use mongodb::bson::doc;
use poise::serenity_prelude::Context;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use crate::OLLAMA;
use crate::config::OLLAMA_MODEL;
use super::SYSTEM_PROMPT;
use crate::database::get_collection;
use mongodb::bson::Bson;
use ollama_rs::generation::chat::{request::ChatMessageRequest, ChatMessage, MessageRole};
use chrono::Utc;

pub(crate) async fn on_mention(
    ctx: &Context,
    msg: &serenity::Message,
    _data: &Data,
    _event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
) -> Result<(), Error> {
    let typing = poise::serenity_prelude::Typing::start(
        ctx.http.clone(),
        msg.channel_id.clone(),
    );

    // Start building prompt
    let mut prompt = mongodb::bson::Document::new();
    prompt.insert("content", Bson::String(msg.content.clone()));
    prompt.insert("author", Bson::Document(doc! {
        "id": msg.author.id.to_string(),
        "name": msg.author.name.clone(),
    }));
    prompt.insert("timestamp", Bson::String(msg.timestamp.to_string()));
    prompt.insert("channel", Bson::Document(doc! {
        "id": msg.channel_id.to_string(),
        "name": msg.channel_id.name(ctx.http.clone()).await?,
    }));
    prompt.insert("guild", Bson::Document(doc! {
        "owner": {
            "id": ctx.cache.guild(msg.guild_id.unwrap()).unwrap().owner_id.to_string() //TODO: add handling if inside of DMs
        }
    }));
    prompt.insert("role", "USER");


    // Detect if the message is a reply
    let mut message_doc_content = doc! {};
    message_doc_content.insert("id", Bson::String(msg.id.to_string()));
    if let Some(referenced) = &msg.referenced_message {
        message_doc_content.insert("replying_to", Bson::Document(doc! {
            "id": referenced.id.to_string(),
        }));
    } else {
        message_doc_content.insert("replying_to", Bson::Null);
    };

    prompt.insert("message", Bson::Document(message_doc_content));

    // See if this guild already has a message collection
    let guild_id = msg.guild_id.unwrap();

    let messages_col = get_collection("messages").expect("Failed to load messages collection");
    let guild_messages = messages_col.find_one(
        doc! {"guild_id": guild_id.to_string()}
    ).await.expect("Failed to query messages collection");

    if guild_messages.is_none() {
        let _ = messages_col.insert_one(doc! {
            "guild_id": guild_id.to_string(),
            "messages": [],
        }).await.expect("Failed to insert new guild into messages collection");

    }

    // Now get the messages collection and insert the system prompt above all other messages
    let messages_col = get_collection("messages").expect("Failed to load messages collection");
    let messages_doc = messages_col.find_one(
        doc! {"guild_id": guild_id.to_string()}
    ).await.expect("Failed to query messages collection").expect("Document somehow got deleted");

    #[allow(unused)]
    let mut messages = messages_doc.get_array("messages")?.clone();

    messages.insert(0, Bson::Document(doc! {
        "content": SYSTEM_PROMPT.to_string(),
        "author": {
            "id": "0".to_string(),
            "name": "System".to_string(),
        },
        "timestamp": Utc::now().timestamp(),
        "channel": doc! {
            "id": "0".to_string(),
            "name": "System".to_string(),
        },
        "guild": doc! {
            "owner": {
                "id": "0".to_string()
            }
        },
        "role": "SYSTEM".to_string(),
    }));

    // Now turn into a ChatHistory
    let mut index: u64 = 0;
    let mut chat_history = messages.to_vec().iter().map(|b| {
        let role = b.as_document().unwrap().get_str("role").unwrap();

        if role == "USER" {
            index += 1;
            ChatMessage::new(MessageRole::User, b.to_string())
        } else if role == "SYSTEM" {
            index += 1;
            ChatMessage::new(MessageRole::System, b.to_string())
        } else if role == "ASSISTANT" {
            index += 1;
            ChatMessage::new(MessageRole::Assistant, b.to_string())
        } else if role == "TOOL"{
            index += 1;
            ChatMessage::new(MessageRole::Tool, b.to_string())
        } else {
            panic!("Invalid role: {}", role);
        }
    }
    ).collect::<Vec<ChatMessage>>(); // ignore the mut, we will not be reusing it. Only ollama_rs needs it to be a mut, instead we will be saving the output.

    let model = OLLAMA_MODEL.get().unwrap().clone();
    let ollama = OLLAMA.get().unwrap().clone();

    let res = ollama
        .send_chat_messages_with_history(
            &mut chat_history,
        ChatMessageRequest::new(model, vec![ChatMessage::new(MessageRole::User, msg.content.clone())])
        ).await?;

    let _ = messages_col.update_one(
        doc! {"guild_id": guild_id.to_string()},
        doc! {"$push": {"messages": prompt}},
    ).await.expect("Failed to update messages collection");

    // Now send the response to the channel
    let msg_id = msg.reply(ctx.http.clone(), res.message.content.clone()).await?.id;

    // Now recreate the ollama response and put it in the db

    let now = Utc::now().timestamp();

    let response_doc = doc! {
        "content": res.message.content.clone(),
        "author": {
            "id": "Assistant",
            "name": "Assistant",
        },
        "timestamp": now,
        "channel": doc! {
            "id": msg.channel_id.to_string(),
            "name": msg.channel_id.name(ctx.http.clone()).await?,
        },
        "guild": doc! {
            "owner": {
                "id": ctx.cache.guild(msg.guild_id.unwrap()).unwrap().owner_id.to_string()
            }
        },
        "role": "ASSISTANT".to_string(),
        "message": doc! {
            "id": msg_id.to_string(),
            "replying_to": doc! {
                "id": msg.id.to_string(),
            }
        }
    };

    let _ = messages_col.update_one(
        doc! {"guild_id": guild_id.to_string()},
        doc! {"$push": {"messages": response_doc}},
    ).await;


    typing.stop();

    Ok(())
}