use serenity::futures::{self, Stream, StreamExt};

use crate::Context;

pub async fn location<'ctx, 'a>(
    ctx: Context<'ctx>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a
where
    'ctx: 'a,
{
    let locations = ctx
        .data()
        .database
        .read()
        .await
        .contents
        .locations
        .clone()
        .into_iter();
    futures::stream::iter(locations)
        .map(|location| location.name.0.clone())
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.to_string())
}

pub async fn watched_satellite<'ctx, 'a>(
    ctx: Context<'ctx>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a
where
    'ctx: 'a,
{
    let watched_satellites = ctx
        .data()
        .database
        .read()
        .await
        .contents
        .watched_satellites
        .clone()
        .into_iter();
    futures::stream::iter(watched_satellites)
        .map(|watched_satellite| watched_satellite.name.clone())
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.to_string())
}
