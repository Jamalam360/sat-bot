mod autocomplete;
mod location;
mod upcoming;
mod watch;

pub use location::*;
use serenity::builder::CreateEmbed;
pub use upcoming::*;
pub use watch::*;

use crate::{n2yo::SatellitePasses, util, Context};

pub fn embed_passes(ctx: &Context<'_>, e: &mut CreateEmbed, passes: SatellitePasses, days: usize) {
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
