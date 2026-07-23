use crate::commands::{build_message_reply, parse_duration};
use crate::{CustomContext, Error};
use poise::CreateReply;
use poise::serenity_prelude::{
    Colour, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, DiscordJsonError, EditMember,
    Error as SError, ErrorResponse, HttpError, Member, Mentionable, StatusCode, Timestamp,
};

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "MODERATE_MEMBERS",
    default_member_permissions = "MODERATE_MEMBERS",
    required_bot_permissions = "MODERATE_MEMBERS",
    guild_only,
    broadcast_typing,
    category = "Moderation",
    description_localized("en-US", "Un-times a member in the server.")
)]
pub(crate) async fn untime(
    ctx: CustomContext<'_>,
    #[description = "Member to un-time"] member: Member,
    #[description = "Reason for the un-time"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    //! Unban a member from the server.
    let reason_pre = reason.unwrap_or_else(|| "No reason provided".to_string());

    let reason = format!("{} | Un-timed by {}", reason_pre, ctx.author().tag());

    match ctx
        .http()
        .as_ref()
        .edit_member(
            member.guild_id,
            member.user.id,
            &EditMember::new().enable_communication(),
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
                        .description("I do not have the required permissions to un-time this user.\n\nThis could mean that the bot's role is lower than the role of the user you are trying to un-time.")
                        .title(":x: Missing Permissions")
                        .timestamp(Timestamp::now())
                        .color(Colour::from_rgb(255, 0, 0)),
                )
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles()); // I don't know if this could happen, but it probably could.

            ctx.send(res).await?;
            return Ok(());
        }
        Err(e) => {
            log::error!("Error in un-timing user: {}", e);
            log::error!("Error Details: {:#?}", e);
            return Err(e.into());
        }
    }

    let res = CreateReply::default()
        .embed(
            CreateEmbed::new()
                .description(format!(
                    "User \"{}\" un-timed \"{}\" for reason \"{}\"",
                    ctx.author().mention(),
                    member.mention(),
                    reason_pre
                ))
                .title("User Un-timed Successfully")
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
