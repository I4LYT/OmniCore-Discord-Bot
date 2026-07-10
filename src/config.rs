use dotenvy::dotenv;
use once_cell::sync::OnceCell;
use std::env;

pub(crate) static DISCORD_TOKEN: OnceCell<String> = OnceCell::new();
pub(crate) static MONGO_URI: OnceCell<String> = OnceCell::new();

pub(crate) fn init_config() {
    dotenv().ok();

    DISCORD_TOKEN
        .set(env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN isn't set."))
        .unwrap();
    MONGO_URI
        .set(env::var("MONGO_URI").expect("MONGO_URI isn't set."))
        .unwrap();
    log::info!("Config initialized!");
}
