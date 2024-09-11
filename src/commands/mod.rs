mod autocomplete;
mod location;
mod upcoming;
mod watch;

pub use location::*;
use serenity::builder::CreateEmbed;
pub use upcoming::*;
pub use watch::*;

use crate::{n2yo::SatellitePasses, util, Context};

pub fn embed_passes(passes: SatellitePasses, days: usize) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!(
            "Upcoming passes for {} in the next {} days",
            passes.info.name, days
        ))
        .fields(passes.passes.iter().map(|pass| {
            (
                util::format_pass_time(pass.start_utc as i64, pass.end_utc as i64),
                format!("Max Elevation: {}°", pass.max_elevation),
                false,
            )
        }))
}
