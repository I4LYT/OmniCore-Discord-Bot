use crate::{CustomContext, Error};
use poise::CreateReply;
use poise::serenity_prelude::{
    Colour, CreateAllowedMentions, CreateEmbed, CreateEmbedAuthor, Member, Mentionable, Role,
    Timestamp,
};

// #[poise::command(
//     slash_command,
//     prefix_command,
//     required_permissions = "MANAGE_CHANNELS",
//     default_member_permissions = "MANAGE_CHANNELS",
//     guild_only,
//     broadcast_typing,
//     category = "Moderation"
// )]
// pub(crate) async fn lock(
//     ctx: CustomContext<'_>,
//     #[description = "Role to place lock role above"] role: Role,
// ) -> Result<(), Error> {
//     //! Locks a channel by placing a lock role above the specified role, applying it to every user, and apply it to the channel.
//

// }
