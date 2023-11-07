use std::sync::Arc;

use poise::{command, serenity_prelude::Channel};
use serenity::{
    builder::{CreateEmbed, CreateMessage},
    http::Http,
    json::Value,
};

use crate::{
    autocomplete,
    database::{Database, Location, LocationName, SatelliteId, Snowflake, WatchedSatellite},
    n2yo::{SatellitePasses, N2YOAPI},
    util, Context,
};

/// Adds an observation location.
#[command(slash_command, rename = "add-location")]
pub async fn add_location(
    ctx: Context<'_>,
    #[description = "name"] name: String,
    #[description = "latitude"] latitude: f64,
    #[description = "longitude"] longitude: f64,
    #[description = "altitude"] altitude: f64,
) -> Result<(), anyhow::Error> {
    ctx.defer().await?;
    let mut database = ctx.data().database.write().await;
    if database
        .contents
        .locations
        .iter()
        .any(|location| location.name.0 == name)
    {
        return Err(anyhow::anyhow!("location already exists"));
    }

    let location = Location {
        name: LocationName(name.clone()),
        creator: Snowflake(ctx.author().id.0),
        latitude,
        longitude,
        altitude,
    };

    database.contents.locations.push(location);
    database.save()?;

    ctx.send(|b| {
        b.embed(|e| {
            e.title("Location added");
            e.description(format!("{} ({})", name, ctx.author().name));
            e
        })
        .ephemeral(false)
    })
    .await?;

    Ok(())
}

/// Lists all observation locations.
#[command(slash_command, rename = "list-locations")]
pub async fn list_locations(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    ctx.defer().await?;
    let database = ctx.data().database.read().await;
    ctx.send(|b| {
        b.embed(|e| {
            e.title("Locations");
            e.fields(database.contents.locations.iter().map(|location| {
                (
                    location.name.0.clone(),
                    format!(
                        "{}°N {}°E @ {}m",
                        location.latitude, location.longitude, location.altitude
                    ),
                    false,
                )
            }));
            e
        })
        .ephemeral(false)
    })
    .await?;

    Ok(())
}

/// Removes an observation location.
#[command(slash_command, rename = "remove-location")]
pub async fn remove_location(
    ctx: Context<'_>,
    #[description = "name"]
    #[autocomplete = "autocomplete::location"]
    name: String,
) -> Result<(), anyhow::Error> {
    ctx.defer().await?;
    let mut database = ctx.data().database.write().await;
    let index = database
        .contents
        .locations
        .iter()
        .position(|location| location.name.0 == name)
        .ok_or_else(|| anyhow::anyhow!("no such location"))?;

    if ctx.author().id.0 != database.contents.locations[index].creator.0 {
        return Err(anyhow::anyhow!("location must be removed by its creator"));
    }
    database.contents.locations.remove(index);
    database.save()?;

    ctx.send(|b| {
        b.embed(|e| {
            e.title("Location removed");
            e.description(format!("{}, created by {}", name, ctx.author().name));
            e
        })
        .ephemeral(false)
    })
    .await?;

    Ok(())
}

/// Gets all the upcoming passes for a satellite.
#[command(slash_command, rename = "get-upcoming-passes")]
pub async fn get_upcoming_passes(
    ctx: Context<'_>,
    #[description = "the NORAD ID of the satellite"] satellite_id: usize,
    #[description = "the location to get passes for"]
    #[autocomplete = "autocomplete::location"]
    location: String,
    #[description = "the number of days in the future to get passes for (max 10)"] days: usize,
    #[description = "the minimum elevation of the passes to get"] min_max_elevation: f64,
) -> Result<(), anyhow::Error> {
    ctx.defer().await?;

    if days > 10 || days == 0 {
        return Err(anyhow::anyhow!("days must be between 1 and 10"));
    }

    if min_max_elevation > 90.0 || min_max_elevation == 0.0 {
        return Err(anyhow::anyhow!(
            "min_max_elevation must be between 1 and 90"
        ));
    }

    let location = {
        let database = ctx.data().database.read().await;
        let location = database
            .contents
            .locations
            .iter()
            .find(|other_location| other_location.name.0 == location)
            .ok_or_else(|| anyhow::anyhow!("no such location"))?;
        location.clone()
    };

    let passes = ctx
        .data()
        .n2yo_api
        .get_satellite_passes(satellite_id, &location, days, min_max_elevation)
        .await?;

    ctx.send(|b| {
        b.embed(|e| {
            embed_passes(&ctx, e, passes, days);
            e
        })
        .ephemeral(false)
    })
    .await?;

    Ok(())
}

/// Gets all the upcoming passes for NOAA 15, 18, and 19.
#[command(slash_command, rename = "get-upcoming-noaa-passes")]
pub async fn get_upcoming_noaa_passes(
    ctx: Context<'_>,
    #[description = "the location to get passes for"]
    #[autocomplete = "autocomplete::location"]
    location: String,
    #[description = "the number of days in the future to get passes for (max 10)"] days: usize,
    #[description = "the minimum elevation of the passes to get"] min_max_elevation: f64,
) -> Result<(), anyhow::Error> {
    ctx.defer().await?;

    if days > 10 || days == 0 {
        return Err(anyhow::anyhow!("days must be between 1 and 10"));
    }

    if min_max_elevation > 90.0 || min_max_elevation == 0.0 {
        return Err(anyhow::anyhow!(
            "min_max_elevation must be between 1 and 90"
        ));
    }

    let location = {
        let database = ctx.data().database.read().await;
        let location = database
            .contents
            .locations
            .iter()
            .find(|other_location| other_location.name.0 == location)
            .ok_or_else(|| anyhow::anyhow!("no such location"))?;
        location.clone()
    };

    let noaa_15_passes = ctx
        .data()
        .n2yo_api
        .get_satellite_passes(25338, &location, days, min_max_elevation)
        .await?;
    let noaa_18_passes = ctx
        .data()
        .n2yo_api
        .get_satellite_passes(28654, &location, days, min_max_elevation)
        .await?;
    let noaa_19_passes = ctx
        .data()
        .n2yo_api
        .get_satellite_passes(33591, &location, days, min_max_elevation)
        .await?;

    ctx.send(|b| {
        b.embed(|e| {
            embed_passes(&ctx, e, noaa_15_passes, days);
            e
        })
        .ephemeral(false);

        b.embed(|e| {
            embed_passes(&ctx, e, noaa_18_passes, days);
            e
        })
        .ephemeral(false);

        b.embed(|e| {
            embed_passes(&ctx, e, noaa_19_passes, days);
            e
        })
        .ephemeral(false);

        b
    })
    .await?;

    Ok(())
}

/// Watch a satellite, sending updates when a suitable pass is identified.
#[command(slash_command, rename = "watch-satellite")]
pub async fn watch_satellite(
    ctx: Context<'_>,
    #[description = "the NORAD ID of the satellite"] satellite_id: usize,
    #[description = "the location to notify of passes for"]
    #[autocomplete = "autocomplete::location"]
    location: String,
    #[description = "the minimum elevation of the passes to notify"] min_max_elevation: f64,
) -> Result<(), anyhow::Error> {
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
pub async fn list_watched_satellites(ctx: Context<'_>) -> Result<(), anyhow::Error> {
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
) -> Result<(), anyhow::Error> {
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
    notify_of_new_passes(&ctx.serenity_context().http).await?;

    ctx.say("Updated watched satellites").await?;

    Ok(())
}

pub async fn notify_of_new_passes(http: &Arc<Http>) -> Result<(), anyhow::Error> {
    let database = Database::open()?;
    let n2yo = N2YOAPI::new()?;
    let mut successful_notifications = Vec::new();

    for watched_satellite in database.contents.watched_satellites.iter() {
        let passes = n2yo
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
        let mut b = CreateMessage::default();

        for pass in passes.passes.iter() {
            if pass.max_elevation >= watched_satellite.min_max_elevation {
                if watched_satellite
                    .previous_notifications
                    .iter()
                    .any(|(start, end)| *start == pass.start_utc && *end == pass.end_utc)
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

        if !passes.passes.is_empty() {
            let mut map = serde_json::Map::new();

            for (key, value) in b.0 {
                map.insert(key.to_string(), value);
            }

            http.send_message(watched_satellite.channel.0, &Value::Object(map))
                .await?;
        }
    }

    // Bored of borrow checker for now.
    drop(database);
    let mut database = Database::open()?;

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

    database.save()?;

    Ok(())
}

fn embed_passes(ctx: &Context<'_>, e: &mut CreateEmbed, passes: SatellitePasses, days: usize) {
    e.title(format!(
        "Upcoming passes for {} in the next {} days",
        passes.info.name, days
    ));
    e.fields(passes.passes.iter().map(|pass| {
        (
            format!(
                "{} - {} ({})",
                util::utc_to_local(ctx.locale().unwrap_or("en-GB"), pass.start_utc as i64),
                util::utc_to_local(ctx.locale().unwrap_or("en-GB"), pass.end_utc as i64),
                util::duration_between(pass.start_utc as i64, pass.end_utc as i64)
            ),
            format!("Max Elevation: {}°", pass.max_elevation),
            false,
        )
    }));
}
