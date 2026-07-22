use super::super::build_message_reply;
use crate::{CustomContext, Error};
use chrono::Utc;
use futures::StreamExt;
use futures::pin_mut;
use mongodb::bson::doc;
use poise::CreateReply;
use poise::serenity_prelude::{
    Channel, Colour, CreateAllowedMentions, CreateEmbed, Member, Mentionable, MessageId, Timestamp,
};

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "MANAGE_MESSAGES",
    default_member_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "MANAGE_MESSAGES",
    guild_only,
    category = "Moderation",
    description_localized(
        "en-US",
        "Purges messages from a channel. /help purge for more information"
    )
)]
pub(crate) async fn purge(
    ctx: CustomContext<'_>,
    #[description = "Channel to purge"] channel: Option<Channel>,
    #[description = "Number of messages to purge"] amount: u64,
    #[description = "Purge from the specified member"] user: Option<Member>,
) -> Result<(), Error> {
    //! Purges messages from a channel.
    //!
    //! Purges messages from a channel.
    //! For messages newer than 14 days, we will still use the bulk delete API.
    //! Messages older than 14 days are deleted individually, since Discord's
    //! bulk delete endpoint rejects/discards anything older than that.
    //! This command takes a while, especially on older messages,
    //! as Discord's API limits individual delete to 5 deletion requests every second. This means that
    //! deleting 100 messages that are older than 14 days will take around 5 minutes.

    // broadcast typing indicator
    let typing = poise::serenity_prelude::Typing::start(ctx.serenity_context().http.clone(), ctx.channel_id());

    if amount == 0 {
        let res = build_message_reply(
            ":x: Invalid Amount",
            "You must specify an amount greater than 0.",
            Colour::from_rgb(255, 0, 0),
            false,
        );
        ctx.send(res).await?;
        return Ok(());
    }

    let amount = amount + 1;

    let http = ctx.http();

    let channel_id = match channel {
        Some(channel) => channel.id(),
        None => ctx.channel_id(),
    };

    let messages = channel_id.messages_iter(&http);
    let mut message_ids: Vec<MessageId> = Vec::new();

    pin_mut!(messages);

    let mut iterated = 0;

    while let Some(message_result) = messages.next().await {
        match message_result {
            Ok(message) => {
                if let Some(user) = &user {
                    if message.author.id != user.user.id {
                        continue;
                    }
                }
                message_ids.push(message.id);
                iterated += 1;
                if iterated >= amount {
                    break;
                }
            }
            Err(error) => log::error!("Failed to get message: {:#?}", error),
        }
    }

    // split messages into bulk and individual deletes based on age
    let cutoff = Utc::now() - chrono::Duration::days(14) + chrono::Duration::minutes(1);
    let (bulk_ids, old_ids): (Vec<MessageId>, Vec<MessageId>) = message_ids
        .into_iter()
        .partition(|id| id.created_at().timestamp() > cutoff.timestamp());

    let mut deleted_count: u64 = 0;
    let mut failed_count: u64 = 0;

    // separate into chunks of 100 messages.
    for chunk in bulk_ids.chunks(100) {
        match channel_id.delete_messages(&http, chunk).await {
            Ok(_) => {
                deleted_count += chunk.len() as u64;
            }
            Err(error) => {
                log::error!("Failed to bulk delete chunk: {:#?}", error);
                failed_count += chunk.len() as u64;
            }
        }
    }

    // Individually delete anything too old for bulk delete.
    // Discord's rate limit here is 5 requests/second, so we space
    // these out to avoid hammering it with 429s.
    let mut delete_interval = tokio::time::interval(std::time::Duration::from_millis(200));

    for id in old_ids {
        delete_interval.tick().await;

        match channel_id.delete_message(&http, id).await {
            Ok(_) => deleted_count += 1,
            Err(error) => {
                log::error!("Failed to delete message {id}: {:#?}", error);
                failed_count += 1;
            }
        }
    }

    #[allow(unused_assignments)]
    let mut description = if failed_count > 0 {
        format!("Deleted {} message(s). Failed to delete {failed_count} message(s).", deleted_count - 1)
    } else {
        format!("Deleted {} message(s).", deleted_count - 1)
    };

    if user.is_some() {
        description = format!(
            "{}\n-# only deleted messages sent by {}",
            description,
            user.unwrap().user.mention()
        );
    }

    if ctx.prefix() == "/" {
        let res = build_message_reply(
            "Purge Complete",
            &description,
            Colour::from_rgb(0, 255, 0),
            false,
        );
        ctx.send(res).await?;
    } else {
        // since the original message has been deleted, we will send a new message that doesn't reply to the deleted one.

        let res = CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .description(format!(
                        "{}\nPurge issued by {}",
                        description,
                        ctx.author().mention()
                    ))
                    .title("Purge Complete")
                    .timestamp(Timestamp::now())
                    .color(Colour::from_rgb(0, 255, 0)),
            )
            .reply(false)
            .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());

        let res = ctx.send(res).await?;
        typing.stop();
        let message = res.message().await?;

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        let _ = message.delete(&http).await;

    }

    Ok(())
}
