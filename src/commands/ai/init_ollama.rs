use crate::OLLAMA;
use crate::config::{OLLAMA_BASE_URL, OLLAMA_MODEL};
use ollama_rs::Ollama;
use reqwest::Client;

pub(crate) async fn init_ollama() {
    // First ping the Ollama server to make sure it's up and running
    let client = Client::new();

    let ollama_base_url = OLLAMA_BASE_URL.get().unwrap();
    let response = client.get(&*ollama_base_url).send().await;
    if let Err(e) = response {
        panic!(
            "Failed to ping Ollama server (check if Ollama is running and public): {}",
            e
        );
    }

    // Create the Ollama client
    let ollama = Ollama::try_new(ollama_base_url)
        .expect("Failed to initialize Ollama client, check your OLLAMA_BASE_URL");
    OLLAMA.set(ollama.clone()).unwrap();

    // List models and validate that the chosen model actually exists
    let ollama_model = OLLAMA_MODEL.get().unwrap();

    let models = ollama
        .list_local_models()
        .await
        .expect("Failed to list Ollama models")
        .iter()
        .map(|m| m.name.clone())
        .collect::<Vec<String>>();

    if !models.contains(&ollama_model) {
        log::error!(
            "Ollama model {} does not exist, check your OLLAMA_MODEL",
            ollama_model
        );
        log::error!("Available models: {:?}", models);
        std::process::exit(78);
    }

    log::info!("Ollama model `{}` is valid", ollama_model);
}
