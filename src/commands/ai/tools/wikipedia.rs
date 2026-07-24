use ollama_rs::generation::tools::Tool;
use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(description = "The topic or article title to search for on Wikipedia")]
    query: String,
}

#[derive(Default)]
pub struct Wikipedia {}

impl Wikipedia {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tool for Wikipedia {
    type Params = Params;

    fn name() -> &'static str {
        "wikipedia_search"
    }

    fn description() -> &'static str {
        "Search Wikipedia and return a plain-text summary of the top matching article. \
         Use this for factual questions about people, places, events, or concepts."
    }

    async fn call(&mut self, params: Self::Params) -> Result<String, Box<dyn Error + Sync + Send>> {
        let params_replaced = params.query.replace(" ", "_");
        let encoded = urlencoding::encode(&params_replaced);
        let url = format!(
            "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
            encoded
        );

        let client = Client::builder()
            .user_agent("OmniCore-Discord-Bot/1.0 (https://steampirate.life)")
            .build()?;

        let resp = client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(format!("No Wikipedia article found for '{}'.", params.query));
        }

        let json: serde_json::Value = resp.json().await?;
        let extract = json
            .get("extract")
            .and_then(|v| v.as_str())
            .unwrap_or("No summary available.");

        Ok(extract.to_string())
    }
}