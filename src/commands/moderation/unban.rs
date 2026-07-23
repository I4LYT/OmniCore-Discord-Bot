use crate::{CustomContext, Error};
use poise::CreateReply;
use poise::serenity_prelude::{
    Colour, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, Mentionable, Timestamp, UserId,
};

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "BAN_MEMBERS",
    default_member_permissions = "BAN_MEMBERS",
    required_bot_permissions = "BAN_MEMBERS",
    guild_only,
    broadcast_typing,
    category = "Moderation",
    description_localized("en-US", "Unbans a member from the server.")
)]
pub(crate) async fn unban(
    ctx: CustomContext<'_>,
    #[description = "Member to unban (mention or user ID)"] member: String,
    #[description = "Reason for the unban"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    //! Unban a member from the server.
    let reason_pre = reason.unwrap_or_else(|| "No reason provided".to_string());

    let reason = format!("{} | Banned by {}", reason_pre, ctx.author().tag());

    let user_id = match parse_user_id(&member) {
        Some(id) => id,
        None => {
            let res = CreateReply::default()
                .embed(
                    CreateEmbed::new()
                        .description("Invalid user ID or mention provided.")
                        .title(":x: Invalid User ID")
                        .timestamp(Timestamp::now())
                        .color(Colour::from_rgb(255, 0, 0)),
                )
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());
            ctx.send(res).await?;
            return Ok(());
        }
    };

    ctx.http()
        .remove_ban(
            ctx.guild_id().unwrap(),
            UserId::from(user_id),
            Some(&*reason),
        )
        .await?; // if error somehow happens, poise error handler will catch it.

    let res = CreateReply::default()
        .embed(
            CreateEmbed::new()
                .description(format!(
                    "User \"{}\" unbanned \"<@{}>\" for reason \"{}\"",
                    ctx.author().mention(),
                    user_id,
                    reason_pre
                ))
                .title("User Unbanned Successfully")
                .timestamp(Timestamp::now())
                .author(CreateEmbedAuthor::new(format!("User ID '{}'", user_id)))
                .color(Colour::from_rgb(0, 255, 0)),
        )
        .reply(true)
        .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());

    ctx.send(res).await?;

    Ok(())
}

// Helper to parse a user ID from a string.
// Returns None if the input is not a valid user ID.
fn parse_user_id(input: &str) -> Option<u64> {
    let input = input.trim();

    if let Some(stripped) = input.strip_prefix("<@") {
        // handles both <@123> and <@!123> (nickname mention variant)
        let stripped = stripped.strip_prefix('!').unwrap_or(stripped);
        let digits = stripped.strip_suffix('>')?;
        return digits.parse::<u64>().ok();
    }

    input.parse::<u64>().ok()
}
