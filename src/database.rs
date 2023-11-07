use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::util;

/// A JSON based database.
pub struct Database {
    pub contents: DatabaseContents,
    path: PathBuf,
}

impl Database {
    pub fn open() -> anyhow::Result<Self> {
        let mut database = Self {
            path: PathBuf::from(util::env("DATABASE_PATH")?),
            contents: DatabaseContents {
                locations: vec![],
                watched_satellites: vec![],
            },
        };

        if !database.path.exists() {
            database.save()?;
        } else {
            database.load()?;
        }

        Ok(database)
    }

    pub fn load(&mut self) -> anyhow::Result<()> {
        let contents = std::fs::read_to_string(&self.path)?;
        self.contents = serde_json::from_str(&contents)?;
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let contents = serde_json::to_string(&self.contents)?;
        std::fs::write(&self.path, contents)?;
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        self.save().unwrap();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseContents {
    pub locations: Vec<Location>,
    pub watched_satellites: Vec<WatchedSatellite>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snowflake(pub u64);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SatelliteId(pub usize);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchedSatellite {
    pub satellite_id: SatelliteId,
    pub name: String,
    pub location: LocationName,
    pub channel: Snowflake,
    pub watcher: Snowflake,
    pub locale: String,
    pub min_max_elevation: f64,
    pub previous_notifications: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationName(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub name: LocationName,
    pub creator: Snowflake,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}
