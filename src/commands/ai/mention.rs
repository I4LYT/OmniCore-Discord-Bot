use super::SYSTEM_PROMPT;
use crate::OLLAMA;
use crate::config::{BOT_OWNERS, OLLAMA_MODEL};
use crate::database::get_collection;
use crate::{Data, Error};
use chrono::Utc;
use mongodb::bson::Bson;
use mongodb::bson::doc;
use mongodb::options::UpdateOptions;
use ollama_rs::generation::chat::{ChatMessage, MessageRole, request::ChatMessageRequest};
use ollama_rs::models::ModelOptions;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Context;

/// Max number of messages to keep in the context window. Excludes sys prompt
const HISTORY_WINDOW: i64 = 40;

pub(crate) async fn on_mention(
    ctx: &Context,
    msg: &serenity::Message,
    _data: &Data,
    _event: &serenity::FullEvent,
    framework: poise::FrameworkContext<'_, Data, Error>,
) -> Result<(), Error> {
    // Don't allow DMs
    let Some(guild_id) = msg.guild_id else {
        msg.reply(ctx.http.clone(), "I only work inside servers, not DMs.")
            .await?;
        return Ok(());
    };

    // Check if the guild is approved for AI usage
    let per_guild_settings_col =
        get_collection("per_guild_settings").expect("Failed to load per_guild_settings collection");
    let guild_settings = per_guild_settings_col
        .find_one(doc! {"guild_id": guild_id.to_string()})
        .await?
        .ok_or("Guild settings not found")?;
    if guild_settings.get_bool("ai_approved").unwrap_or(false) == false {
        let owners = BOT_OWNERS
            .get()
            .unwrap()
            .clone()
            .into_iter()
            .map(|id| format!("<@{}>", id))
            .collect::<Vec<String>>()
            .join(", ");
        msg.reply(ctx.http.clone(), format!("This server is not approved for AI usage.\nYou can DM the users below with your Guild ID and ask to approve AI usage:\n{}\nYour Guild ID is `{}`", owners, guild_id.to_string())).await?;
        return Ok(());
    }

    let typing = poise::serenity_prelude::Typing::start(ctx.http.clone(), msg.channel_id);

    let guild_owner_id = ctx
        .cache
        .guild(guild_id)
        .map(|g| g.owner_id.to_string())
        .unwrap_or_default();

    let channel_name = msg
        .channel_id
        .name(ctx.http.clone())
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    // Build the doc representing the current triggering message.
    let mut prompt = mongodb::bson::Document::new();
    prompt.insert("content", Bson::String(msg.content.clone()));
    prompt.insert(
        "author",
        Bson::Document(doc! {
            "id": msg.author.id.to_string(),
            "name": msg.author.name.clone(),
        }),
    );
    prompt.insert("timestamp", Bson::String(msg.timestamp.to_string()));
    prompt.insert(
        "channel",
        Bson::Document(doc! {
            "id": msg.channel_id.to_string(),
            "name": channel_name.clone(),
        }),
    );
    prompt.insert(
        "guild",
        Bson::Document(doc! {
            "owner": { "id": guild_owner_id.clone() }
        }),
    );
    prompt.insert("role", "USER");

    let mut message_doc_content = doc! { "id": msg.id.to_string() };
    if let Some(referenced) = &msg.referenced_message {
        message_doc_content.insert("replying_to", doc! { "id": referenced.id.to_string() });
    } else {
        message_doc_content.insert("replying_to", Bson::Null);
    }
    prompt.insert("message", Bson::Document(message_doc_content));

    let messages_col = get_collection("messages").expect("Failed to load messages collection");

    // Atomically ensure the guild doc exists and push the new message in one round trip.
    // $setOnInsert only applies when the upsert creates a new doc, so no race with concurrent
    // mentions in a guild that has no doc yet.
    messages_col
        .update_one(
            doc! { "guild_id": guild_id.to_string() },
            doc! {
                "$setOnInsert": { "guild_id": guild_id.to_string() },
                "$push": { "messages": prompt.clone() },
            },
        )
        .with_options(UpdateOptions::builder().upsert(true).build())
        .await?;

    // Fetch back only the last HISTORY_WINDOW messages via $slice, keeping context bounded.
    let messages_doc = messages_col
        .find_one(doc! { "guild_id": guild_id.to_string() })
        .projection(doc! { "messages": { "$slice": -HISTORY_WINDOW } })
        .await?
        .ok_or("Guild message document missing immediately after upsert")?;

    let mut messages = messages_doc.get_array("messages")?.clone();

    let custom_system_prompt = guild_settings.get_str("ai_prompt").unwrap_or_default();

    messages.insert(
            0,
            Bson::Document(doc! {
            "content": SYSTEM_PROMPT.to_string().replace("<BOT_USER_ID>", &framework.bot_id.to_string()).replace("<CUSTOM_SYSTEM_PROMPT>", &custom_system_prompt),
            "author": {
                "id": "0".to_string(),
                "name": "System".to_string(),
            },
            "timestamp": Utc::now().timestamp(),
            "channel": {
                "id": "0".to_string(),
                "name": "System".to_string(),
            },
            "guild": {
                "owner": { "id": "0".to_string() }
            },
            "role": "SYSTEM".to_string(),
        }),
        );

    // Only USER turns carry the full JSON envelope (author/channel/guild metadata the model
    // needs to act correctly). SYSTEM and ASSISTANT turns are plain text — if assistant replies
    // are stored back into history as JSON docs, the model starts pattern-matching on its own
    // prior turns and echoes JSON back to users instead of plain text, no matter what the system
    // prompt instructs. TOOL turns keep the structured form since tool output is inherently data.
    let mut chat_history = messages
        .iter()
        .map(|b| {
            let doc = b.as_document().unwrap();
            let role = doc.get_str("role").unwrap();
            match role {
                "USER" => Ok(ChatMessage::new(MessageRole::User, b.to_string())),
                "SYSTEM" => {
                    let content = doc.get_str("content").unwrap_or_default().to_string();
                    Ok(ChatMessage::new(MessageRole::System, content))
                }
                "ASSISTANT" => {
                    let content = doc.get_str("content").unwrap_or_default().to_string();
                    Ok(ChatMessage::new(MessageRole::Assistant, content))
                }
                "TOOL" => Ok(ChatMessage::new(MessageRole::Tool, b.to_string())),
                other => Err(format!("Invalid role in stored message: {}", other)),
            }
        })
        .collect::<Result<Vec<ChatMessage>, String>>()?;

    // The current message is already the last entry in `messages` (we pushed it above), so it's
    // already part of `chat_history`. Pass an empty new-turn request — ollama_rs still needs a
    // request object, but the actual prompt content lives in history now, keeping the model's
    // view of the current message consistent (same JSON envelope) with every prior turn.
    let model = OLLAMA_MODEL.get().unwrap().clone();
    let ollama = OLLAMA.get().unwrap().clone();

    let res = ollama
        .send_chat_messages_with_history(
            &mut chat_history,
            ChatMessageRequest::new(model, vec![])
                .options(ModelOptions::default().num_ctx(8192).num_predict(512)),
        )
        .await;

    typing.stop();

    let res = match res {
        Ok(res) => res,
        Err(e) => {
            log::error!("Ollama request failed for guild {}: {}", guild_id, e);
            msg.reply(
                ctx.http.clone(),
                "Sorry, I couldn't process that — something went wrong on my end.",
            )
            .await?;
            return Err(e.into());
        }
    };

    let mut message = res.message.content.clone();
    if message.trim().is_empty() {
        message = "AI returned no response.".to_string();
    }

    // Try to reply (threads the response off the original message). If the original message was
    // deleted in the meantime, Discord rejects the message_reference and serenity surfaces that
    // as an error — fall back to a plain channel send instead of failing the whole handler.
    let sent = match msg.reply(ctx.http.clone(), &message).await {
        Ok(sent) => sent,
        Err(e) => {
            log::warn!(
                "Reply failed (original message likely deleted) in channel {}: {}. Falling back to plain send.",
                msg.channel_id,
                e
            );
            msg.channel_id.say(ctx.http.clone(), &message).await?
        }
    };

    let now = Utc::now().timestamp();
    let response_doc = doc! {
        "content": &message,
        "author": {
            "id": "Assistant",
            "name": "Assistant",
        },
        "timestamp": now,
        "channel": {
            "id": msg.channel_id.to_string(),
            "name": channel_name,
        },
        "guild": {
            "owner": { "id": guild_owner_id }
        },
        "role": "ASSISTANT".to_string(),
        "message": {
            "id": sent.id.to_string(),
            "replying_to": { "id": msg.id.to_string() }
        }
    };

    if let Err(e) = messages_col
        .update_one(
            doc! { "guild_id": guild_id.to_string() },
            doc! { "$push": { "messages": response_doc } },
        )
        .await
    {
        // Don't fail the handler over this — the reply already went out — but log loudly since
        // it means the next mention's context will be missing this assistant turn.
        log::error!(
            "Failed to persist assistant response for guild {}: {}",
            guild_id,
            e
        );
    }

    Ok(())
}
