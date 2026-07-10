use mongodb::{
    bson::doc,
    options::ClientOptions,
    Client,
};
use once_cell::sync::{OnceCell};

pub static CLIENT: OnceCell<mongodb::Client> = OnceCell::new();

pub async fn ensure_indexes() -> mongodb::error::Result<()> {

    Ok(())
}

pub async fn mongo_connect() -> mongodb::error::Result<()> {
    let mongodb_uri = crate::config::MONGO_URI.get().unwrap();

    let client_options = ClientOptions::parse(mongodb_uri).await
        .map_err(|e| mongodb::error::Error::custom(format!("Invalid MongoDB URI: {}", e)))?;
    let client = Client::with_options(client_options)?;
    client.database("admin").run_command(doc! { "ping": 1 }).await?;

    CLIENT.set(client).map_err(|_| {
        mongodb::error::Error::custom("MongoDB client already initialized")
    })?;

    log::info!("Connected to MongoDB!");
    Ok(())
}

pub fn get_collection(
    collection_name: &str
) -> Result<mongodb::Collection<mongodb::bson::Document>, mongodb::error::Error> {
    let client = CLIENT.get().ok_or_else(|| {
        mongodb::error::Error::custom("MongoDB client not initialized")
    })?;
    Ok(client
        .database("omnicore_bot")
        .collection::<mongodb::bson::Document>(collection_name))
}


pub async fn mongo_shutdown() {
    log::info!("Attempting MongoDB shutdown...");

    let client = match CLIENT.get() {
        Some(c) => c.clone(),
        None => {
            log::warn!("mongo_shutdown() called but MongoDB was never initialized.");
            return;
        }
    };

    client.shutdown().await;

    log::info!("MongoDB shutdown complete.");
}
