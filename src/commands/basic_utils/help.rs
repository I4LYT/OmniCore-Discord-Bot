use crate::{CustomContext, Error};


#[poise::command(
    prefix_command,
    track_edits,
    slash_command,
    description_localized("en-US", "Help command that lists all available commands.")
)]
pub async fn help(
    ctx: CustomContext<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
    //! Help command that lists all available commands.
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
Type ?help command for more info on a command.
You can edit your message to the bot and the bot will edit its response.",
        include_description: true,
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}