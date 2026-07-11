// ping command
use super::super::build_message_reply;
use crate::{CustomContext, Error};
use poise::serenity_prelude::Colour;

#[poise::command(
    slash_command,
    prefix_command,
    description_localized("en-US", "Checks the ping, along with some other useful information.")
)]
pub(crate) async fn ping(ctx: CustomContext<'_>) -> Result<(), Error> {
    //! Checks the ping, along with some other useful information.
    let ping = ctx.ping().await;

    let desc = format!(
        "
- Ping: {}ms
- Shard: `{}`
        ",
        ping.as_millis(),
        ctx.serenity_context().shard_id.0,
    );
    let res = build_message_reply("Pong!", &*desc, Colour::from_rgb(88, 101, 242), false);
    ctx.send(res).await?;
    Ok(())
}
