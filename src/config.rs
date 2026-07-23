use dotenvy::dotenv;
use once_cell::sync::OnceCell;
use std::env;

pub(crate) static DISCORD_TOKEN: OnceCell<String> = OnceCell::new();
pub(crate) static MONGO_URI: OnceCell<String> = OnceCell::new();
pub(crate) static OLLAMA_BASE_URL: OnceCell<String> = OnceCell::new();
pub(crate) static OLLAMA_MODEL: OnceCell<String> = OnceCell::new();

pub(crate) fn init_config() {
    dotenv().ok();

    DISCORD_TOKEN
        .set(env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN isn't set."))
        .unwrap();
    MONGO_URI
        .set(env::var("MONGO_URI").expect("MONGO_URI isn't set."))
        .unwrap();
    OLLAMA_BASE_URL
        .set(env::var("OLLAMA_BASE_URL").expect("OLLAMA_BASE_URL isn't set."))
        .unwrap();
    OLLAMA_MODEL
        .set(env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL isn't set."))
        .unwrap();

    if OLLAMA_BASE_URL.get().unwrap().ends_with("/") {
        log::error!("OLLAMA_BASE_URL must not end with a slash");
        std::process::exit(78); // 78 is the exit code for config errors
    }

    log::info!("Config initialized!");
}
