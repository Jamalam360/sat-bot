use poise::command;

use crate::commands::{autocomplete, embed_passes, Context};

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
) -> anyhow::Result<()> {
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

    if !passes.passes.is_empty() {
        ctx.send(|b| {
            b.embed(|e| {
                embed_passes(&ctx, e, passes, days);
                e
            })
            .ephemeral(false)
        })
        .await?;
    } else {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("No passes found");
                e
            })
            .ephemeral(false)
        })
        .await?;
    }

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
) -> anyhow::Result<()> {
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

    if noaa_15_passes.passes.is_empty()
        && noaa_18_passes.passes.is_empty()
        && noaa_19_passes.passes.is_empty()
    {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("No passes found");
                e
            })
            .ephemeral(false)
        })
        .await?;
        return Ok(());
    }
    
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
