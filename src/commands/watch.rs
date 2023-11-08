use std::sync::Arc;

use poise::command;
use poise::serenity_prelude::Channel;
use serenity::builder::CreateMessage;
use serenity::http::Http;
use serenity::json::Value;
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
                && watched_satellite.channel.0 == ctx.channel_id().0
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
        channel: Snowflake(ctx.channel_id().0),
        watcher: Snowflake(ctx.author().id.0),
        locale: ctx.locale().unwrap_or("en-GB").to_string(),
        location: LocationName(location.name.0.clone()),
        name: name.clone(),
        min_max_elevation,
        previous_notifications: Vec::new(),
    });
    database.save()?;

    ctx.send(|b| {
        b.embed(|e| {
            e.title("Satellite watched");
            e.description(format!(
                "{} with a minimum elevation of {}° at {} by {}",
                name,
                min_max_elevation,
                location.name.0,
                ctx.author().name,
            ));
            e
        })
        .ephemeral(false)
    })
    .await?;

    Ok(())
}

/// Lists all watched satellites.
#[command(slash_command, rename = "list-watched-satellites")]
pub async fn list_watched_satellites(ctx: Context<'_>) -> anyhow::Result<()> {
    ctx.defer().await?;

    let database = ctx.data().database.read().await;
    ctx.send(|b| {
        b.embed(|e| {
            e.title("Watched satellites");
            e.fields(
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
            );
            e
        })
        .ephemeral(false)
    })
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
                && watched_satellite.channel.0 == channel.id().0
                && watched_satellite.location.0 == location
        })
        .ok_or_else(|| anyhow::anyhow!("no such watched satellite"))?;

    if ctx.author().id.0 != database.contents.watched_satellites[index].watcher.0 {
        return Err(anyhow::anyhow!(
            "watched satellite must be removed by its watcher"
        ));
    }

    database.contents.watched_satellites.remove(index);
    database.save()?;

    ctx.send(|b| {
        b.embed(|e| {
            e.title("Watched satellite removed");
            e.description(format!(
                "{} ({})",
                database.contents.watched_satellites[index].name,
                ctx.author().name
            ));
            e
        })
        .ephemeral(false)
    })
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

        if passes.passes.len() == 0 {
            continue;
        }

        let mut b = CreateMessage::default();

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

                b.add_embed(|e| {
                    e.title(format!(
                        "Upcoming pass for {} at {}",
                        passes.info.name, watched_satellite.location.0
                    ));

                    e.description(format!(
                        "{} - {} ({})\nMax Elevation: {}°",
                        util::utc_to_local(&watched_satellite.locale, pass.start_utc as i64),
                        util::utc_to_local(&watched_satellite.locale, pass.end_utc as i64),
                        util::duration_between(pass.start_utc as i64, pass.end_utc as i64),
                        pass.max_elevation
                    ));
                    e
                });
            }
        }

        let mut map = serde_json::Map::new();
        for (key, value) in b.0 {
            map.insert(key.to_string(), value);
        }

        http.send_message(watched_satellite.channel.0, &Value::Object(map))
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

    let current_utc = util::current_utc();
    database
        .contents
        .watched_satellites
        .iter_mut()
        .for_each(|ws| {
            ws.previous_notifications = ws
                .previous_notifications
                .iter()
                .filter(|(start, end)| {
                    current_utc - 24 * 60 * 60 < *start as i64
                        && current_utc - 24 * 60 * 60 < *end as i64
                })
                .cloned()
                .collect();
        });

    database.save()?;

    Ok(())
}
