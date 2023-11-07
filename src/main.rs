use std::time::Duration;

use database::Database;
use n2yo::N2YOAPI;
use poise::{serenity_prelude::GuildId, FrameworkError};
use serenity::prelude::*;
use tokio::{spawn, time::interval};

mod autocomplete;
mod commands;
mod database;
mod n2yo;
mod util;

pub struct ApplicationContext {
    pub database: RwLock<Database>,
    pub n2yo_api: N2YOAPI,
}

pub type Context<'a> = poise::Context<'a, ApplicationContext, anyhow::Error>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    util::load_env_file()?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::add_location(),
                commands::list_locations(),
                commands::remove_location(),
                commands::get_upcoming_passes(),
                commands::get_upcoming_noaa_passes(),
                commands::watch_satellite(),
                commands::list_watched_satellites(),
                commands::unwatch_satellite(),
                commands::update_watched_satellites(),
            ],
            on_error: |error| {
                Box::pin(async move {
                    println!("Error: {}", error);
                    match error {
                        FrameworkError::Command {
                            error: actual_error,
                            ctx,
                        } => {
                            let _ = ctx
                                .send(|b| {
                                    b.embed(|e| {
                                        e.title("Error");
                                        e.description(format!("```{}```", actual_error));
                                        e
                                    })
                                    .ephemeral(false)
                                })
                                .await;
                        }
                        _ => {
                            if let Some(ctx) = error.ctx() {
                                let _ = ctx
                                    .send(|b| {
                                        b.embed(|e| {
                                            e.title("Error");
                                            e.description(format!("```{}```", error));
                                            e
                                        })
                                        .ephemeral(false)
                                    })
                                    .await;
                            } else {
                                println!("No error context");
                            }
                        }
                    }
                })
            },
            ..Default::default()
        })
        .token(util::env("DISCORD_TOKEN")?)
        .intents(GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild::<ApplicationContext, anyhow::Error>(
                    ctx,
                    &framework.options().commands,
                    GuildId(util::env("GUILD_ID")?.parse()?),
                )
                .await?;
                Ok(ApplicationContext {
                    database: Database::open()?.into(),
                    n2yo_api: N2YOAPI::new()?,
                })
            })
        })
        .build()
        .await?;

    let http = framework.client().cache_and_http.http.clone();

    spawn(async move {
        let mut interval = interval(Duration::from_secs(60 * 60 * 30));

        loop {
            interval.tick().await;
            let _ = commands::notify_of_new_passes(&http).await;
        }
    });

    framework.start().await?;

    Ok(())
}
