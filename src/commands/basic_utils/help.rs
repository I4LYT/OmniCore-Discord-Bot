use crate::{CustomContext, Error, commands::basic_utils::prefix::get_prefix};

#[poise::command(
    prefix_command,
    track_edits,
    slash_command,
    description_localized("en-US", "Help command that lists all available commands."),
    broadcast_typing
)]
pub async fn help(
    ctx: CustomContext<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
    //! Help command that lists all available commands.

    let prefix = get_prefix(ctx.guild_id().unwrap_or(poise::serenity_prelude::GuildId::new(1))).await;

    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: &format!(
            "Type {}help command for more info on a command. Mention (@) the bot to talk to the AI.",
            prefix
        ),
        include_description: true,
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}
