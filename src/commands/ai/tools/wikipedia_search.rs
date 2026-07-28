/// Tool for AI to search Wikipedia.
use ollama_rs::generation::tools::Tool;
use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(description = "The topic or article title to search for on Wikipedia")]
    query: String,
}

#[derive(Default)]
pub struct WikipediaSearch {}

impl WikipediaSearch {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Deserialize)]
struct WikipediaSearchResult {
    pages: Vec<WikipediaPage>,
}

#[derive(Deserialize)]
struct WikipediaPage {
    key: String,
    title: String,
    description: Option<String>,
}

impl Display for WikipediaSearchResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for page in &self.pages {
            writeln!(
                f,
                "Key: {}, Title: {}, Description: {}",
                page.key,
                page.title,
                page.description.as_deref().unwrap_or("No description")
            )?;
        }
        Ok(())
    }
}

impl Tool for WikipediaSearch {
    type Params = Params;

    fn name() -> &'static str {
        "wikipedia_search"
    }

    fn description() -> &'static str {
        "Search Wikipedia by a query and get a list containing descriptions, titles, and keys of the 20 first results."
    }

    async fn call(&mut self, params: Self::Params) -> Result<String, Box<dyn Error + Sync + Send>> {
        let encoded = urlencoding::encode(&params.query);

        let url = format!(
            "https://en.wikipedia.org/w/rest.php/v1/search/title?q={}&limit=20",
            encoded
        );

        let client = Client::builder()
            .user_agent("OmniCore-Discord-Bot/1.0 (https://steampirate.life)")
            .build()?;

        let resp = client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(format!(
                "Error while searching for Wikipedia Articles '{:#?}'.",
                resp
            ));
        }

        let result: WikipediaSearchResult = resp.json().await?;
        
        if result.to_string().is_empty() {
            return Ok(format!("No Wikipedia articles found for '{}'.", params.query));
        }
        
        Ok(result.to_string())
    }
}
