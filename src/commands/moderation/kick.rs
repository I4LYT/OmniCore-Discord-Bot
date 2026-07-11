use crate::{CustomContext, Error};
use poise::CreateReply;
use poise::serenity_prelude::{
    Colour, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, Member, Mentionable, Timestamp,
};

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "KICK_MEMBERS",
    default_member_permissions = "KICK_MEMBERS",
    guild_only,
    broadcast_typing
)]
pub(crate) async fn kick(
    ctx: CustomContext<'_>,
    #[description = "Member to kick"] member: Member,
    #[description = "Reason for the kick"] reason: Option<String>,
) -> Result<(), Error> {
    let reason_pre = reason.unwrap_or_else(|| "No reason provided".to_string());

    let reason = format!("{} | Kicked by {}", reason_pre, ctx.author().tag());

    member.kick_with_reason(&ctx.http(), &reason).await?;

    let res = CreateReply::default()
        .embed(
            CreateEmbed::new()
                .description(format!(
                    "User \"{}\" kicked \"{}\" for reason \"{}\"",
                    ctx.author().mention(),
                    member.mention(),
                    reason_pre
                ))
                .title("User Kicked Successfully")
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
