use crate::{CustomContext, Error};
use poise::CreateReply;
use poise::serenity_prelude::{
    Colour, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, DiscordJsonError,
    Error as SError, ErrorResponse, HttpError, Member, Mentionable, StatusCode, Timestamp,
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
) -> Result<(), Error> {
    //! Unban a member from the server.
    
    
    
    Ok(())
}