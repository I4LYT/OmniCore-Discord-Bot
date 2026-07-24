use super::super::build_message_reply;
use crate::{CustomContext, Error, database::get_collection, setup_guild};
use mongodb::bson::doc;
use poise::serenity_prelude::Colour;

#[poise::command(
    slash_command,
    prefix_command,
    description_localized("en-US", "Gets the current system prompt for the bot."),
    guild_only,
    broadcast_typing,
    category = "AI"
)]
pub(crate) async fn get_prompt(ctx: CustomContext<'_>) -> Result<(), Error> {
    //! Gets the current system prompt for the bot
    let per_guild_settings_col =
        get_collection("per_guild_settings").expect("Failed to load per_guild_settings collection");

    if per_guild_settings_col
        .find_one(doc! {"guild_id": ctx.guild_id().unwrap().to_string()})
        .await?
        .is_none()
    {
        setup_guild(ctx.guild_id().unwrap()).await;
    }

    let per_guild_settings = per_guild_settings_col
        .find_one(doc! {"guild_id": ctx.guild_id().unwrap().to_string()})
        .await?
        .unwrap();

    let prompt = per_guild_settings
        .get_str("ai_prompt")
        .unwrap_or("Not Set Yet. Use `change_prompt` to set a prompt.");

    ctx.send(build_message_reply(
        "Current Prompt",
        &format!(
            "The current system prompt for this server is:\n\n```{}```",
            prompt
        ),
        Colour::from_rgb(0, 255, 0),
        false,
    ))
    .await?;

    return Ok(());
}
