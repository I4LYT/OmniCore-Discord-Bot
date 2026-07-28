/// Tool for AI to query Wikipedia.
use ollama_rs::generation::tools::Tool;
use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(description = "The key to get the Wikipedia article for")]
    key: String,
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
        "wikipedia_get"
    }

    fn description() -> &'static str {
        "Get a Wikipedia article by the key and return a plain-text summary of the article. \
         Use this for factual questions about people, places, events, or concepts.\
         First use the wikipedia_search tool to find the key for the article you want to get.\
         This must NOT contain any spaces."
    }

    async fn call(&mut self, params: Self::Params) -> Result<String, Box<dyn Error + Sync + Send>> {
        if params.key.contains(" ") {
            return Ok(format!(
                "Please remove any spaces from the query '{}'.",
                params.key
            ));
        }
        let encoded = urlencoding::encode(&params.key);
        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&prop=extracts&explaintext=true&titles={}&format=json",
            encoded
        );

        let client = Client::builder()
            .user_agent("OmniCore-Discord-Bot/1.0 (https://steampirate.life)")
            .build()?;

        let resp = client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(format!("No Wikipedia article found for '{}'.", params.key));
        }

        let json: serde_json::Value = resp.json().await?;
        let extract = json
            .get("query")
            .and_then(|q| q.get("pages"))
            .and_then(|p| p.as_object())
            .and_then(|o| o.values().next())
            .and_then(|v| v.get("extract"))
            .and_then(|e| e.as_str())
            .unwrap_or("No extract found for this article.");

        Ok(extract.to_string())
    }
}
