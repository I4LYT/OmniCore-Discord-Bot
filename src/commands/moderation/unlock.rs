use super::super::build_message_reply;
use crate::database::get_collection;
use crate::{CustomContext, Error};
use mongodb::bson::doc;
use poise::CreateReply;
use poise::serenity_prelude::{
    Channel, Colour, CreateAllowedMentions, CreateEmbed, DiscordJsonError, Error as SError,
    ErrorResponse, HttpError, RoleId, StatusCode, Timestamp,
};

#[poise::command(
    slash_command,
    prefix_command,
    required_bot_permissions = "MANAGE_ROLES | MANAGE_CHANNELS",
    required_permissions = "MANAGE_CHANNELS",
    default_member_permissions = "MANAGE_CHANNELS",
    guild_only,
    broadcast_typing,
    category = "Moderation",
    description_localized("en-US", "Unlocks a channel. /help unlock for more information")
)]
pub(crate) async fn unlock(
    ctx: CustomContext<'_>,
    #[description = "Channel to unlock"] channel: Channel,
) -> Result<(), Error> {
    //! Unlockes a channel.
    //!
    //! Unlocks a channel by deleting the lock role.
    let guild_id = ctx
        .guild_id()
        .ok_or("This command must be used in a guild")?;

    let locked_channels_col =
        get_collection("locked_channels").expect("Failed to load locked_channels collection");

    let lock_doc = match locked_channels_col
        .find_one(doc! {
            "guild_id": guild_id.get().to_string(),
            "channel_id": channel.id().get().to_string(),
        })
        .await?
    {
        Some(d) => d,
        None => {
            ctx.send(build_message_reply(
                ":x: Channel not locked",
                "This channel is not locked.",
                Colour::from_rgb(255, 0, 0),
                false,
            ))
            .await?;
            return Ok(());
        }
    };

    // Verify that the role even exists
    let lock_role = match guild_id
        .role(
            &ctx.http(),
            RoleId::new(lock_doc.get_str("role_id").unwrap().parse().unwrap()),
        )
        .await
    {
        Ok(r) => r,
        Err(SError::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
            status_code: StatusCode::NOT_FOUND,
            error: DiscordJsonError { code: 10011, .. }, // Role not found
            ..
        }))) => {
            let res = CreateReply::default()
                .embed(
                    CreateEmbed::new()
                        .description("The lock role created by OmniCore has been deleted.\n\nThis means someone has tampered with the lock role, so the target channel should already be unlocked.")
                        .title(":x: Lock Role Tampered With")
                        .timestamp(Timestamp::now())
                        .color(Colour::from_rgb(255, 0, 0)),
                )
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());

            ctx.send(res).await?;
            return Ok(());
        }
        Err(e) => {
            log::error!("Error in finding role: {}", e);
            log::error!("Error Details: {:#?}", e);
            return Err(e.into());
        }
    };

    // Now delete the role and remove the document from the database
    let _ = guild_id.delete_role(&ctx.http(), lock_role.id).await?;

    let _ = locked_channels_col.delete_one(lock_doc).await?;

    return Ok(());
}
