use crate::commands::{build_message_reply, parse_duration};
use crate::{CustomContext, Error};
use poise::CreateReply;
use poise::serenity_prelude::{
    Colour, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, DiscordJsonError, EditMember,
    Error as SError, ErrorResponse, HttpError, Member, Mentionable, StatusCode, Timestamp,
};

const MAX_TIMEOUT_SECS: u64 = 28 * 24 * 60 * 60; // 28 days

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "MODERATE_MEMBERS",
    default_member_permissions = "MODERATE_MEMBERS",
    required_bot_permissions = "MODERATE_MEMBERS",
    guild_only,
    broadcast_typing,
    category = "Moderation",
    description_localized("en-US", "Times a member in the server.")
)]
pub(crate) async fn time(
    ctx: CustomContext<'_>,
    #[description = "Member to timeout"] member: Member,
    #[description = "Duration, e.g. 20m, 2days, 1week"] duration: String,
    #[description = "Reason for the timeout"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    //! Unban a member from the server.
    let reason_pre = reason.unwrap_or_else(|| "No reason provided".to_string());

    let reason = format!("{} | Timed by {}", reason_pre, ctx.author().tag());

    // Parse the duration string into a chrono::Duration
    let parsed = match parse_duration(&duration) {
        Ok(d) => d,
        Err(_) => {
            let res = build_message_reply(
                ":x: Invalid Duration",
                "Couldn't parse that duration. Try something like `20m`, `2days`, or `1week`.",
                Colour::RED,
                false,
            );
            ctx.send(res).await?;
            return Ok(());
        }
    };

    if parsed.as_secs() > MAX_TIMEOUT_SECS {
        let res = build_message_reply(
            ":x: Invalid Duration",
            "Timeouts can't exceed 28 days.",
            Colour::RED,
            false,
        );
        ctx.send(res).await?;
        return Ok(());
    }

    if parsed.as_secs() == 0 {
        let res = build_message_reply(
            ":x: Invalid Duration",
            "Duration must be greater than zero.",
            Colour::RED,
            false,
        );
        ctx.send(res).await?;
        return Ok(());
    }

    let until = chrono::Utc::now() + chrono::Duration::seconds(parsed.as_secs() as i64);

    match ctx
        .http()
        .as_ref()
        .edit_member(
            member.guild_id,
            member.user.id,
            &EditMember::new().disable_communication_until(until.to_rfc3339()),
            Some(&reason),
        )
        .await
    {
        Ok(_) => {}
        Err(SError::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
            status_code: StatusCode::FORBIDDEN,
            error: DiscordJsonError { code: 50013, .. },
            ..
        }))) => {
            let res = CreateReply::default()
                .embed(
                    CreateEmbed::new()
                        .description("I do not have the required permissions to time this user.\n\nThis could mean that the bot's role is lower than the role of the user you are trying to time.")
                        .title(":x: Missing Permissions")
                        .timestamp(Timestamp::now())
                        .color(Colour::from_rgb(255, 0, 0)),
                )
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());

            ctx.send(res).await?;
            return Ok(());
        }
        Err(e) => {
            log::error!("Error in timing user: {}", e);
            log::error!("Error Details: {:#?}", e);
            return Err(e.into());
        }
    }

    let res = CreateReply::default()
        .embed(
            CreateEmbed::new()
                .description(format!(
                    "User \"{}\" timed \"{}\" for reason \"{}\"",
                    ctx.author().mention(),
                    member.mention(),
                    reason_pre
                ))
                .title("User Timed Successfully")
                .timestamp(Timestamp::now())
                .author(
                    CreateEmbedAuthor::new(member.display_name()).icon_url(
                        member
                            .user
                            .avatar_url()
                            .unwrap_or_else(|| member.user.default_avatar_url()),
                    ),
                )
                .color(Colour::from_rgb(0, 255, 0)),
        )
        .reply(true)
        .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());

    ctx.send(res).await?;

    Ok(())
}
