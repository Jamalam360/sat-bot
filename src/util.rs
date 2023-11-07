use std::path::PathBuf;

use anyhow::Context as _;
use chrono::FixedOffset;

pub fn load_env_file() -> anyhow::Result<()> {
    let path = PathBuf::from(".env");

    if path.exists() {
        for line in std::fs::read_to_string(&path)?.lines() {
            if line.trim().starts_with('#') || line.trim().is_empty() {
                continue;
            }

            let mut parts = line.splitn(2, '=');
            let name = parts.next().unwrap().trim();
            let value = parts.next().unwrap().trim();
            std::env::set_var(name, value);
        }
    }

    Ok(())
}

pub fn env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("missing environment variable {}", name))
}

pub fn utc_to_local(locale: &str, utc: i64) -> String {
    let local = chrono::DateTime::from_timestamp(utc, 0)
        .unwrap()
        .with_timezone(&locale_to_timezone(locale));
    local.format("%d/%m/%Y %H:%M").to_string()
}

/// Imperfect, but good enough for my usecase.
fn locale_to_timezone(locale: &str) -> FixedOffset {
    match locale {
        "en-US" => chrono::FixedOffset::east_opt(5 * 3600),
        "en-GB" => chrono::FixedOffset::east_opt(0),
        "en-AU" => chrono::FixedOffset::east_opt(10 * 3600),
        _ => chrono::FixedOffset::east_opt(0),
    }
    .unwrap()
}

pub fn duration_between(a: i64, b: i64) -> String {
    let duration = chrono::Duration::seconds(b - a);
    let days = duration.num_days();
    let hours = duration.num_hours() - days * 24;
    let minutes = duration.num_minutes() - days * 24 * 60 - hours * 60;
    let seconds = duration.num_seconds() - days * 24 * 60 * 60 - hours * 60 * 60 - minutes * 60;

    format!("{}m {}s", minutes, seconds)
}
