use poise::command;

use crate::{
    commands::autocomplete,
    database::{Location, LocationName, Snowflake},
    Context,
};

/// Adds an observation location.
#[command(slash_command, rename = "add-location")]
pub async fn add_location(
    ctx: Context<'_>,
    #[description = "name"] name: String,
    #[description = "latitude"] latitude: f64,
    #[description = "longitude"] longitude: f64,
    #[description = "altitude"] altitude: f64,
) -> anyhow::Result<()> {
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
pub async fn list_locations(ctx: Context<'_>) -> anyhow::Result<()> {
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
) -> anyhow::Result<()> {
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
