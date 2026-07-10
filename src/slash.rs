// ping command
use crate::{get_guild_name, CustomContext, Error};
use poise::serenity_prelude::GuildId;
use poise::{
    serenity_prelude::{
        Colour, CreateEmbed, Timestamp,
    },
    CreateReply,
};

#[poise::command(
    slash_command,
    prefix_command,
    description_localized("en-US", "Checks the ping, along with some other useful information.")
)]
pub(crate) async fn ping(ctx: CustomContext<'_>) -> Result<(), Error> {
    let ping = ctx.ping().await;

    let prefix = crate::set_prefix::get_prefix(ctx.guild_id().unwrap_or(GuildId::new(1))).await;

    let desc = format!(
        "
- Ping: {}ms
- Server: {}
- Shard: `{}`
- Bot Prefix for this server: `{}`
- GitHub: https://github.com/Shreshtgaming606/OmniCore-Discord-Bot/
        ",
        ping.as_millis(),
        get_guild_name(&ctx).await,
        ctx.serenity_context().shard_id.0,
        prefix
    );
    let res = CreateReply::default().embed(
        CreateEmbed::new()
            .description(desc)
            .title("Bot Status")
            .timestamp(Timestamp::now())
            .color(Colour::from_rgb(88, 101, 242)),
    ).reply(true);
    ctx.send(res).await?;
    Ok(())
}
