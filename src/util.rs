use std::path::PathBuf;

use anyhow::Context as _;
use tracing::info;

pub fn load_env_file() -> anyhow::Result<()> {
    info!("Loading .env file");
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

pub fn format_pass_time(start: i64, end: i64) -> String {
    format!(
        "<t:{}> - <t:{}:t> ({})",
        start,
        end,
        duration_between(start, end)
    )
}

pub fn duration_between(a: i64, b: i64) -> String {
    let duration = chrono::Duration::seconds(b - a);
    let days = duration.num_days();
    let hours = duration.num_hours() - days * 24;
    let minutes = duration.num_minutes() - days * 24 * 60 - hours * 60;
    let seconds = duration.num_seconds() - days * 24 * 60 * 60 - hours * 60 * 60 - minutes * 60;

    format!("{}m {}s", minutes, seconds)
}

pub fn are_within_10_seconds(a: i64, b: i64) -> bool {
    let duration = chrono::Duration::seconds(b - a);
    duration.num_seconds().abs() < 10
}

pub fn current_utc() -> i64 {
    chrono::Utc::now().timestamp()
}
