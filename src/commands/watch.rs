use std::sync::Arc;

use poise::serenity_prelude::Channel;
use poise::{command, CreateReply};
use serenity::all::{ChannelId, CreateEmbed};
use serenity::builder::CreateMessage;
use serenity::http::Http;
use tokio::sync::RwLock;

use crate::{
    commands::autocomplete,
    database::{Database, LocationName, SatelliteId, Snowflake, WatchedSatellite},
    n2yo::N2YOAPI,
    util, Context,
};

/// Watch a satellite, sending updates when a suitable pass is identified.
#[command(slash_command, rename = "watch-satellite")]
pub async fn watch_satellite(
    ctx: Context<'_>,
    #[description = "the NORAD ID of the satellite"] satellite_id: usize,
    #[description = "the location to notify of passes for"]
    #[autocomplete = "autocomplete::location"]
    location: String,
    #[description = "the minimum elevation of the passes to notify"] min_max_elevation: f64,
) -> anyhow::Result<()> {
    ctx.defer().await?;

    if min_max_elevation > 90.0 || min_max_elevation == 0.0 {
        return Err(anyhow::anyhow!(
            "min_max_elevation must be between 1 and 90"
        ));
    }

    let mut database = ctx.data().database.write().await;

    if database
        .contents
        .watched_satellites
        .iter()
        .any(|watched_satellite| {
            watched_satellite.satellite_id.0 == satellite_id
                && watched_satellite.location.0 == location
                && watched_satellite.min_max_elevation == min_max_elevation
                && watched_satellite.channel.0 == ctx.channel_id().get()
        })
    {
        return Err(anyhow::anyhow!(
            "satellite already being watched in this channel with these parameters"
        ));
    }

    let location = {
        let location = database
            .contents
            .locations
            .iter()
            .find(|other_location| other_location.name.0 == location)
            .ok_or_else(|| anyhow::anyhow!("no such location"))?;
        location.clone()
    };

    let name = ctx
        .data()
        .n2yo_api
        .get_name_from_norad_id(satellite_id)
        .await?;

    database.contents.watched_satellites.push(WatchedSatellite {
        satellite_id: SatelliteId(satellite_id),
        channel: Snowflake(ctx.channel_id().get()),
        watcher: Snowflake(ctx.author().id.get()),
        __legacy_locale: ctx.locale().unwrap_or("en-GB").to_string(),
        location: LocationName(location.name.0.clone()),
        name: name.clone(),
        min_max_elevation,
        previous_notifications: Vec::new(),
    });
    database.save()?;

    ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title("Satellite watched")
                    .description(format!(
                        "{} with a minimum elevation of {}° at {} by {}",
                        name,
                        min_max_elevation,
                        location.name.0,
                        ctx.author().name,
                    )),
            )
            .ephemeral(false),
    )
    .await?;

    Ok(())
}

/// Lists all watched satellites.
#[command(slash_command, rename = "list-watched-satellites")]
pub async fn list_watched_satellites(ctx: Context<'_>) -> anyhow::Result<()> {
    ctx.defer().await?;

    let database = ctx.data().database.read().await;

    ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::new().title("Watched Satellites").fields(
                    database
                        .contents
                        .watched_satellites
                        .iter()
                        .map(|watched_satellite| {
                            (
                                watched_satellite.name.clone(),
                                format!(
                                    "Channel: {}\nLocation: {}\nMinimum Elevation: {}°",
                                    watched_satellite.channel.0,
                                    watched_satellite.location.0,
                                    watched_satellite.min_max_elevation
                                ),
                                false,
                            )
                        }),
                ),
            )
            .ephemeral(false),
    )
    .await?;

    Ok(())
}

/// Removes a watched satellite.
#[command(slash_command, rename = "unwatch-satellite")]
pub async fn unwatch_satellite(
    ctx: Context<'_>,
    #[description = "the NORAD ID of the satellite"]
    #[autocomplete = "autocomplete::watched_satellite"]
    satellite_id: usize,
    #[description = "the channel the satellite is being watched in"] channel: Channel,
    #[description = "the location the satellite is being watched from"]
    #[autocomplete = "autocomplete::location"]
    location: String,
) -> anyhow::Result<()> {
    ctx.defer().await?;
    let mut database = ctx.data().database.write().await;
    let index = database
        .contents
        .watched_satellites
        .iter()
        .position(|watched_satellite| {
            watched_satellite.satellite_id.0 == satellite_id
                && watched_satellite.channel.0 == channel.id().get()
                && watched_satellite.location.0 == location
        })
        .ok_or_else(|| anyhow::anyhow!("no such watched satellite"))?;

    if ctx.author().id.get() != database.contents.watched_satellites[index].watcher.0 {
        return Err(anyhow::anyhow!(
            "watched satellite must be removed by its watcher"
        ));
    }

    database.contents.watched_satellites.remove(index);
    database.save()?;

    ctx.send(
        CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title("Watched Satellite Removed")
                    .description(format!(
                        "{} ({})",
                        database.contents.watched_satellites[index].name,
                        ctx.author().name
                    )),
            )
            .ephemeral(false),
    )
    .await?;

    Ok(())
}

/// Update watched satellites.
#[command(slash_command, rename = "update-watched-satellites")]
pub async fn update_watched_satellites(ctx: Context<'_>) -> anyhow::Result<()> {
    ctx.defer().await?;
    notify_of_new_passes(
        &ctx.serenity_context().http,
        &ctx.data().database,
        &ctx.data().n2yo_api,
    )
    .await?;

    ctx.say("Updated watched satellites").await?;

    Ok(())
}

pub async fn notify_of_new_passes(
    http: &Arc<Http>,
    database: &Arc<RwLock<Database>>,
    n2yo_api: &Arc<N2YOAPI>,
) -> anyhow::Result<()> {
    let mut successful_notifications = Vec::new();
    let mut database = database.write().await;

    for watched_satellite in database.contents.watched_satellites.iter() {
        let passes = n2yo_api
            .get_satellite_passes(
                watched_satellite.satellite_id.0,
                database
                    .contents
                    .locations
                    .iter()
                    .find(|location| location.name.0 == watched_satellite.location.0)
                    .unwrap(),
                1,
                watched_satellite.min_max_elevation,
            )
            .await?;

        if passes.passes.is_empty() {
            continue;
        }

        let mut builder = CreateMessage::default();

        for pass in passes.passes.iter() {
            if pass.max_elevation >= watched_satellite.min_max_elevation {
                if watched_satellite
                    .previous_notifications
                    .iter()
                    .any(|(start, end)| {
                        util::are_within_10_seconds(*start as i64, pass.start_utc as i64)
                            && util::are_within_10_seconds(*end as i64, pass.end_utc as i64)
                    })
                {
                    continue;
                } else {
                    successful_notifications.push((
                        watched_satellite.satellite_id.0,
                        pass.start_utc,
                        pass.end_utc,
                    ));
                }

                builder = builder.add_embed(
                    CreateEmbed::new()
                        .title(format!(
                            "Upcoming pass for {} at {}",
                            passes.info.name, watched_satellite.location.0
                        ))
                        .description(format!(
                            "{}\nMax Elevation: {}°",
                            util::format_pass_time(pass.start_utc as i64, pass.end_utc as i64),
                            pass.max_elevation
                        )),
                );
            }
        }

        http.send_message(
            ChannelId::new(watched_satellite.channel.0),
            vec![],
            &builder,
        )
        .await?;
    }

    for successful in successful_notifications.iter() {
        database
            .contents
            .watched_satellites
            .iter_mut()
            .find(|watched_satellite| watched_satellite.satellite_id.0 == successful.0)
            .unwrap()
            .previous_notifications
            .push((successful.1, successful.2));
    }

    database
        .contents
        .watched_satellites
        .iter_mut()
        .for_each(|ws| {
            ws.previous_notifications.retain(|(start, end)| {
                !has_more_than_one_day_passed(*start as i64)
                    && !has_more_than_one_day_passed(*end as i64)
            });
        });

    database.save()?;

    Ok(())
}

fn has_more_than_one_day_passed(since: i64) -> bool {
    util::current_utc() - since > 24 * 60 * 60
}
