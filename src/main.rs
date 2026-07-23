mod commands;
mod config;
mod database;
mod logging;

use mongodb::bson::doc;
use once_cell::sync::OnceCell;
use poise::{
    Command, CreateReply, FrameworkError,
    serenity_prelude::{self as serenity, Colour, CreateEmbed, GuildId, Timestamp},
};
use serenity::async_trait;
use serenity::gateway::ActivityData;
use serenity::model::gateway::Ready;
use serenity::model::user::OnlineStatus;
use serenity::prelude::*;
use std::collections::HashSet;
use tokio::signal;
use tokio::signal::unix::{SignalKind, signal};

#[derive(Clone, Debug, Copy)]
struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type CustomContext<'a> = poise::Context<'a, Data, Error>;
struct Handler;

static START_TIME: OnceCell<chrono::DateTime<chrono::Utc>> = OnceCell::new();

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        log::info!(
            "Shard {} is connected to {}",
            ready.shard.unwrap().id,
            ready.user.name
        );
    }
}

async fn get_user_name(ctx: &CustomContext<'_>) -> String {
    if ctx.guild_id().is_none() {
        return ctx.author().name.clone();
    }
    ctx.author_member().await.map_or_else(
        || ctx.author().name.clone(),
        |member| member.user.name.clone(),
    )
}

async fn get_guild_name(ctx: &CustomContext<'_>) -> String {
    if ctx.guild_id().is_none() {
        return "DMs (not an actual server)".to_string();
    }
    ctx.guild().unwrap().name.clone()
}

#[allow(dead_code)]
async fn get_guild_owner_id(ctx: &CustomContext<'_>) -> serenity::UserId {
    if ctx.guild_id().is_none() {
        return serenity::UserId::new(1); // Invalid, so nothing can point to it.
    }
    ctx.guild().unwrap().owner_id
}

#[tokio::main]
async fn main() {
    START_TIME
        .set(chrono::Utc::now())
        .expect("Failed to set START_TIME");

    logging::init_logging();
    log::info!("Starting OmniCore Discord Bot...");
    config::init_config();
    let _ = database::mongo_connect()
        .await
        .expect("Failed to connect to MongoDB");
    let _ = database::ensure_indexes()
        .await
        .expect("Failed to ensure indexes");

    let cmds: Vec<Command<Data, Box<dyn std::error::Error + Send + Sync>>> = vec![
        commands::basic_utils::ping::ping(),
        commands::basic_utils::prefix::set_prefix(),
        commands::basic_utils::info::info(),
        commands::basic_utils::help::help(),
        commands::moderation::kick::kick(),
        commands::moderation::ban::ban(),
        commands::moderation::unban::unban(),
        commands::moderation::lock::lock(),
        commands::moderation::unlock::unlock(),
        commands::moderation::purge::purge(),
        commands::moderation::time::time()
    ];

    let token = config::DISCORD_TOKEN.get().unwrap();
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::privileged()
        | GatewayIntents::non_privileged()
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::AUTO_MODERATION_EXECUTION
        | GatewayIntents::AUTO_MODERATION_CONFIGURATION;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            owners: HashSet::from([serenity::UserId::new(1157083515486220429)]),
            commands: cmds,
            command_check: Some(|ctx| {
                Box::pin(async move {
                    log::info!("Checking command {} by {} in {}", ctx.command().qualified_name, get_user_name(&ctx).await, ctx.guild_id().unwrap_or(GuildId::new(1)));
                    Ok(true)
                })
            }),

            on_error: |err| {
                //TODO: Might make this auto-report to telemetry system and also give an option for people self-hosting the bot to turn this off
                Box::pin(async move {
                    #[allow(unused)]
                    let mut skip = false;
                    #[allow(unused)]
                    let mut invalid_args = false;

                    match err {
                        FrameworkError::CommandCheckFailed {error: _, ctx: _, ..} => {skip = true}, // to prevent double logging
                        FrameworkError::ArgumentParse {error: _, ctx: _, ..} => {invalid_args = true},
                        FrameworkError::UnknownCommand {..} => {skip = true}, // to prevent double logging
                        _ => {}
                    }

                    if invalid_args {
                        let _ = err.ctx().unwrap().send(CreateReply::default().embed(
                            CreateEmbed::new()
                                .description(format!("Failed to parse arguments, please check the command usage by using the `help` command followed by the command name. e.g. `help info`\n{}", err.to_string().replace("`", "'")))
                                .title(":x: Failed to Parse Arguments")
                                .timestamp(Timestamp::now())
                                .color(Colour::RED),
                        ).reply(true).ephemeral(true)).await;
                        return;
                    }

                    if err.ctx().is_none() && !skip {
                        log::error!("Error while handling command (context is not available): {:#?}", err);
                    } else if !skip {
                        log::error!("Error while handling command: {:#?}", err);
                        let _ = err.ctx().unwrap().send(CreateReply::default().embed(
                            CreateEmbed::new()
                                .description(format!("There was an error while processing your command: \n ```{}```\nPlease report this issue to https://github.com/Shreshtgaming606/OmniCore-Discord-Bot", err.to_string().replace("`", "'")) )
                                .title(":x: Internal (sometimes user) Error")
                                .timestamp(Timestamp::now())
                                .color(Colour::RED),
                        ).reply(true).ephemeral(true)).await;
                    }
                })
            },
            pre_command: |ctx| {
                        Box::pin(async move {
                            log::info!("Executing command {} by {} in {}", ctx.command().qualified_name, get_user_name(&ctx).await, ctx.guild_id().unwrap_or(GuildId::new(1)));
                        })
                    },
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: None,
                mention_as_prefix: false,
                dynamic_prefix: Some(|ctx| { // Dynamic prefix, so it can be changed per server
                    Box::pin(async move {
                        if ctx.guild_id.is_none() { // is_none() is true if the command was executed in a DM
                            Ok(Some("!".to_string()))
                        } else {
                            let guild = ctx.guild_id.unwrap();
                            let prefix = commands::basic_utils::prefix::get_prefix(guild).await;
                            Ok(Some(prefix.to_owned()))
                        }
                    })
                }),
                ..Default::default()
            },
            ..Default::default()
            })
            .setup(|ctx, ready, framework| {
                Box::pin(async move {
                    log::info!("Started OmniCore Discord Bot!");
                    log::info!("{} is in {} servers", ready.user.name, ctx.http.get_guilds(None, None).await.unwrap().len());
                    ctx.shard.set_presence(
                        Some(ActivityData::custom("/help | OmniCore Discord Bot")),
                        OnlineStatus::Online
                    );
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(Data {})
                })
            })
            .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await;

    let mut unwrapped_client = client.unwrap();
    let shard_manager = unwrapped_client.shard_manager.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        shard_manager.shutdown_all().await;
        database::mongo_shutdown().await;
        log::info!("Bot has been shutdown!");
    });
    unwrapped_client.start_shards(4).await.unwrap(); //TODO: Make this configurable with a default value of 2
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {print!("\n"); log::info!("Stopping bot (CTRL+C)...");}
        _ = terminate => {log::info!("Stopping bot (SIGTERM)...");},
    }
}

async fn setup_guild(guild: GuildId) {
    let guild_id = guild.get();
    let per_guild_settings_col = database::get_collection("per_guild_settings")
        .expect("Failed to load per_guild_settings collection");

    let guild_doc = doc! {
        "guild_id": guild_id.to_string(),
        "prefix": "!" // Default prefix
    };

    if per_guild_settings_col
        .find_one(doc! {"guild_id": guild_id.to_string()})
        .await
        .unwrap()
        .is_none()
    {
        let _ = per_guild_settings_col.insert_one(guild_doc).await;
    }
}
