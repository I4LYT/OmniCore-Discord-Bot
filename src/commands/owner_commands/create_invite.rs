use crate::commands::build_message_reply;
use crate::{CustomContext, Error};
use poise::serenity_prelude::{ChannelType, CreateInvite};
use poise::{
    CreateReply,
    serenity_prelude::{
        Colour, ComponentInteractionCollector, CreateActionRow, CreateEmbed,
        CreateInteractionResponse, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
        GuildId,
    },
};
use std::time::Duration;

#[poise::command(
    slash_command,
    prefix_command,
    description_localized("en-US", "Creates an invite for the specified server."),
    dm_only,
    owners_only,
    category = "Bot Owner Utilities"
)]
pub async fn create_invite(ctx: CustomContext<'_>) -> Result<(), Error> {
    let typing = poise::serenity_prelude::Typing::start(
        ctx.serenity_context().http.clone(),
        ctx.channel_id(),
    );

    let guilds = ctx
        .http()
        .get_guilds(None, None)
        .await?
        .into_iter()
        .map(|guild| guild.id)
        .collect::<Vec<_>>();

    if guilds.is_empty() {
        build_message_reply(
            ":x: Not in any servers",
            "This bot is not in any servers.",
            Colour::from_rgb(255, 0, 0),
            false,
        );
        return Ok(());
    }

    // Discord select menus cap out at 25 options.
    let options: Vec<CreateSelectMenuOption> = guilds
        .iter()
        .take(25)
        .filter_map(|guild_id| {
            ctx.cache()
                .guild(*guild_id)
                .map(|guild| CreateSelectMenuOption::new(guild.name.clone(), guild_id.to_string()))
        })
        .collect();

    let select_menu = CreateSelectMenu::new(
        "create_invite_guild_select",
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Choose a server...");

    let action_row = CreateActionRow::SelectMenu(select_menu);

    let reply = ctx
        .send(
            CreateReply::default()
                .embed(CreateEmbed::new().title("Select a server to create an invite for").description("Select a server from the dropdown below and you will be generated an invite link.").color(Colour::from_rgb(88, 101, 242)))
                .components(vec![action_row])
                .ephemeral(true),
        )
        .await?;

    typing.stop();

    // Wait for the owner to pick something.
    let interaction = ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(Duration::from_secs(60))
        .filter(|mci| mci.data.custom_id == "create_invite_guild_select")
        .await;

    let Some(interaction) = interaction else {
        reply
            .edit(
                ctx.into(),
                CreateReply::default()
                    .content("Timed out waiting for a selection.")
                    .components(vec![]),
            )
            .await?;
        return Ok(());
    };

    let selected_guild_id = match &interaction.data.kind {
        poise::serenity_prelude::ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned()
        }
        _ => None,
    };

    let typing = poise::serenity_prelude::Typing::start(
        ctx.serenity_context().http.clone(),
        ctx.channel_id(),
    );
    // Acknowledge the interaction so Discord doesn't show "This interaction failed".
    interaction
        .create_response(ctx.http(), CreateInteractionResponse::Acknowledge)
        .await?;

    let Some(guild_id_str) = selected_guild_id else {
        return Ok(());
    };

    ctx.say(format!("You selected guild ID: {}", guild_id_str))
        .await?;

    let guild_id = match guild_id_str.parse::<u64>() {
        Ok(id) => GuildId::new(id),
        Err(_) => {
            ctx.say("Invalid guild ID selected.").await?;
            typing.stop();
            return Ok(());
        }
    };

    // Just use the first text channel you can get, invites will still let you join
    let channels = guild_id.channels(ctx.http()).await?;

    let first_channel = match channels.values().find(|c| c.kind == ChannelType::Text) {
        Some(channel) => channel,
        None => {
            ctx.say("No channels found in this guild.").await?;
            typing.stop();
            return Ok(());
        }
    };

    let invite = match first_channel
        .create_invite(
            ctx.http(),
            CreateInvite::default()
                .max_age(3600) // 1 hour
                .max_uses(1) // 1 use
                .unique(true),
        )
        .await
    {
        Ok(invite) => invite,
        Err(error) => {
            ctx.say(&format!("Failed to create invite: \n ```{:?}```", error))
                .await?;
            typing.stop();
            return Ok(());
        }
    };

    let invite_url = invite.url();

    ctx.say(format!("Invite created: {}", invite_url)).await?;
    typing.stop();

    Ok(())
}
