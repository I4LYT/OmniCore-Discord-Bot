pub(crate) mod ai;
pub(crate) mod basic_utils;
pub(crate) mod moderation;

use poise::{
    CreateReply,
    serenity_prelude::{Colour, CreateAllowedMentions, CreateEmbed, Timestamp},
};
use regex::Regex;
use std::time::Duration;

fn build_message_reply(title: &str, desc: &str, color: Colour, mention: bool) -> CreateReply {
    let res = CreateReply::default()
        .embed(
            CreateEmbed::new()
                .description(desc)
                .title(title)
                .timestamp(Timestamp::now())
                .color(color),
        )
        .reply(true);

    if !mention {
        return res.allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());
    }

    return res;
}

#[derive(Debug)]
pub enum DurationParseError {
    Empty,
    NoMatch,
    Overflow,
}

pub fn parse_duration(input: &str) -> Result<Duration, DurationParseError> {
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return Err(DurationParseError::Empty);
    }

    // Matches: number + unit, e.g. "20", "m" / "20minutes" / "2 weeks"
    let re = Regex::new(r"(\d+)\s*([a-z]+)").unwrap();

    let mut total_secs: u64 = 0;
    let mut matched = false;

    for cap in re.captures_iter(&input) {
        matched = true;
        let value: u64 = cap[1].parse().map_err(|_| DurationParseError::Overflow)?;
        let unit = &cap[2];

        let secs_per_unit: u64 = match unit {
            "s" | "sec" | "secs" | "second" | "seconds" => 1,
            "m" | "min" | "mins" | "minute" | "minutes" => 60,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3600,
            "d" | "day" | "days" => 86400,
            "w" | "wk" | "wks" | "week" | "weeks" => 604800,
            _ => return Err(DurationParseError::NoMatch),
        };

        total_secs = total_secs
            .checked_add(
                value
                    .checked_mul(secs_per_unit)
                    .ok_or(DurationParseError::Overflow)?,
            )
            .ok_or(DurationParseError::Overflow)?;
    }

    if !matched {
        return Err(DurationParseError::NoMatch);
    }

    Ok(Duration::from_secs(total_secs))
}
