pub(crate) mod basic_utils;

use poise::{
    CreateReply,
    serenity_prelude::{Colour, CreateAllowedMentions, CreateEmbed, Timestamp},
};

fn build_message_reply(title: &str, desc: &str, color: Colour, mention: bool) -> CreateReply {
    let res = CreateReply::default()
        .embed(
            CreateEmbed::new()
                .description(desc)
                .title(title)
                .timestamp(Timestamp::now())
                .color(color),
        )
        .reply(true);

    if !mention {
        return res.allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());
    }

    return res;
}
