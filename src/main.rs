use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use database::Database;
use n2yo::N2YOAPI;
use poise::{serenity_prelude::GuildId, FrameworkError};
use serenity::prelude::*;
use tokio::{spawn, sync::RwLock, time::interval};
use tracing::{error, info};

mod commands;
mod database;
mod n2yo;
mod util;

pub struct ApplicationContext {
    pub database: Arc<RwLock<Database>>,
    pub n2yo_api: Arc<N2YOAPI>,
}

pub type Context<'a> = poise::Context<'a, ApplicationContext, anyhow::Error>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    util::load_env_file()?;

    let database = Arc::new(RwLock::new(Database::open()?));
    let n2yo_api = Arc::new(N2YOAPI::new()?);

    let app_ctx = ApplicationContext {
        database: database.clone(),
        n2yo_api: n2yo_api.clone(),
    };

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
            on_error,
            ..Default::default()
        })
        .token(util::env("DISCORD_TOKEN")?)
        .intents(GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                info!("Registering commands");
                poise::builtins::register_in_guild::<ApplicationContext, anyhow::Error>(
                    ctx,
                    &framework.options().commands,
                    GuildId(util::env("GUILD_ID")?.parse()?),
                )
                .await?;

                Ok(app_ctx)
            })
        })
        .build()
        .await?;

    let http = framework.client().cache_and_http.http.clone();

    spawn(async move {
        let mut interval = interval(Duration::from_secs(60 * 60 * 30));

        loop {
            info!("Waiting for next interval");
            interval.tick().await;
            info!("Checking for new passes");
            let _ = commands::notify_of_new_passes(&http, &database, &n2yo_api).await;
        }
    });

    info!("Starting bot");
    framework.start().await?;

    Ok(())
}

fn on_error<'a>(
    framework_error: FrameworkError<'a, ApplicationContext, anyhow::Error>,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        info!("Handling an error");

        let message = match &framework_error {
            FrameworkError::Setup { error, .. } => {
                error!("Encountered error during setup: {}", error);
                None
            }
            FrameworkError::EventHandler { error, .. } => {
                error!("Encountered error during event handling: {}", error);
                None
            }
            FrameworkError::Command { error, .. } => {
                error!("Encountered error during command handling: {}", error);
                Some(format!("Error: {}", error))
            }
            FrameworkError::SubcommandRequired { .. } => {
                error!("Command invoked without a subcommand");
                Some("Command invoked without a subcommand".to_string())
            }
            FrameworkError::CommandPanic { payload, .. } => {
                error!(
                    "Command panicked: {}",
                    payload.clone().unwrap_or("".to_string())
                );
                Some("Command panicked".to_string())
            }
            FrameworkError::ArgumentParse { error, .. } => {
                error!("Encountered error parsing arguments: {}", error);
                Some(format!("Error parsing arguments: {}", error))
            }
            FrameworkError::CommandStructureMismatch { description, .. } => {
                error!("Command structure mismatch: {}", description);
                None
            }
            FrameworkError::UnknownInteraction { interaction, .. } => {
                error!("Unknown interaction: {:?}", interaction);
                Some("Unknown interaction".to_string())
            }
            _ => unreachable!(),
        };

        if let Some(ctx) = framework_error.ctx() {
            let _ = ctx
                .send(|b| {
                    b.embed(|e| {
                        e.title("An error occurred");
                        e.description(message.unwrap_or("Unknown error".to_string()));
                        e
                    })
                    .ephemeral(false)
                })
                .await;
        }
    })
}
