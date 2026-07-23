use super::super::build_message_reply;
use crate::{CustomContext, Error, database::get_collection, setup_guild};
use mongodb::bson::doc;
use poise::serenity_prelude::{Colour, GuildId};

#[poise::command(
    slash_command,
    prefix_command,
    description_localized("en-US", "Approves AI usage in the server"),
    owners_only,
    broadcast_typing,
    category = "AI"
)]
pub(crate) async fn approve(
    ctx: CustomContext<'_>,
    #[description = "Guild ID to approve"] guild_id: Option<GuildId>,
) -> Result<(), Error> {
    //! Approves AI usage in the server
    let per_guild_settings_col =
        get_collection("per_guild_settings").expect("Failed to load per_guild_settings collection");

    let mut guild_id = guild_id;

    if guild_id.is_none() {
        if ctx.guild().is_none() {
            ctx.send(build_message_reply(
                ":x: Missing Guild ID",
                "Please provide a guild ID to approve AI usage in.",
                Colour::RED,
                false,
            ))
            .await?;
        } else {
            guild_id = Some(ctx.guild_id().unwrap());
        }
    }

    if per_guild_settings_col
        .find_one(doc! {"guild_id": guild_id.unwrap().to_string()})
        .await?
        .is_none()
    {
        setup_guild(guild_id.unwrap()).await;
    }

    let _ = per_guild_settings_col
        .update_one(
            doc! {"guild_id": guild_id.unwrap().to_string()},
            doc! {"$set": {"ai_approved": true}},
        )
        .await;

    ctx.send(build_message_reply(
        "Approved AI Usage",
        &format!(
            "Successfully updated AI approval status for guild `{}`",
            guild_id.unwrap().to_string()
        ),
        Colour::from_rgb(0, 255, 0),
        false,
    ))
    .await?;

    return Ok(());
}
