use super::super::get_highest_role_from_member;
use crate::{CustomContext, Error};
use poise::CreateReply;
use poise::serenity_prelude::{
    Colour, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, Member, Mentionable, Timestamp,
};

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    description_localized("en-US", "Gets the highest role from a member."),
    broadcast_typing,
    category = "Utility"
)]
pub async fn highest_role_from_member(
    ctx: CustomContext<'_>,
    #[description = "Member to get the highest role from"] member: Member,
) -> Result<(), Error> {
    let highest_role = get_highest_role_from_member(&member, ctx).unwrap();

    let username = member.clone().user.name;
    let avatar_url = member.user.face();

    let res = CreateReply::default()
        .embed(
            CreateEmbed::new()
                .description(format!(
                    "{}'s highest role is {}",
                    member.mention(),
                    highest_role.mention()
                ))
                .timestamp(Timestamp::now())
                .author(CreateEmbedAuthor::new(username).icon_url(avatar_url))
                .color(Colour::from_rgb(88, 101, 242)),
        )
        .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles())
        .reply(true);

    ctx.send(res).await?;

    Ok(())
}
