/// Tool for AI to timeout people for aggressive behavior.
use ollama_rs::generation::tools::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use std::error::Error;
use crate::commands::moderation::time::MAX_TIMEOUT_SECS;
use crate::commands::{parse_duration};
use poise::serenity_prelude::{GuildId, Context, CacheHttp, EditMember, Message};

#[derive(Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(description = "The duration of the timeout (max of 28 days) (e.g. 1d12h or 1min, 2h, 30s)")]
    duration: String,
    #[schemars(description = "The reason for the timeout")]
    reason: String,
}

pub struct Timeout {
    context: Context,
    message: Message,
    guild_id: GuildId,
}

impl Timeout {
    pub fn new(ctx: Context, message: Message, guild_id: GuildId) -> Self {
        Self { context: ctx, message, guild_id }
    }
}

impl Tool for Timeout {
    type Params = Params;

    fn name() -> &'static str {
        "timeout"
    }

    fn description() -> &'static str {
        "Timeouts the user you are talking to for a specified duration. \
         Use this to temporarily remove the user's ability to communicate in the server.\
         It is imperative that you still respond to the user's message, saying that they have been timed out. "
    }

    async fn call(&mut self, params: Self::Params) -> Result<String, Box<dyn Error + Sync + Send>> {
        let duration = params.duration.clone();
        // Parse the duration string into a chrono::Duration
        let parsed = match parse_duration(&duration) {
            Ok(d) => d,
            Err(_) => {
                return Err("Invalid duration. Try something like `20m`, `2days`, or `1week`.".into());
            }
        };

        if parsed.as_secs() > MAX_TIMEOUT_SECS {
            return Err("Timeouts can't exceed 28 days.".into());
        }

        if parsed.as_secs() == 0 {
            return Err("Duration must be greater than zero.".into());
        }

        let until = chrono::Utc::now() + chrono::Duration::seconds(parsed.as_secs() as i64);
        let ctx = &self.context;

        let result = ctx.http()
            .as_ref()
            .edit_member(
                self.guild_id,
                self.message.author.id,
                &EditMember::new().disable_communication_until(until.to_rfc3339()),
                Some(&params.reason),
            )
            .await;

        if let Err(e) = result {
            return Err(format!(
                "Failed to timeout user {}: {}. Params were: duration={}, reason={}",
                self.message.author.id, e, params.duration, params.reason
            ).into());
        }

        let extract = format!(
            "User {} has been timed out for {}. Reason: {}",
            self.message.author.id, params.duration, params.reason
        );

        Ok(extract.to_string())
    }
}