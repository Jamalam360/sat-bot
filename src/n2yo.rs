use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{database::Location, util};

pub struct N2YOAPI {
    api_key: String,
    client: reqwest::Client,
}

impl N2YOAPI {
    pub fn new() -> anyhow::Result<Self> {
        info!("Creating N2YO API client");
        Ok(Self {
            api_key: util::env("N2YO_KEY")?,
            client: reqwest::ClientBuilder::new()
                .user_agent("sat-bot (james@jamalam.tech)")
                .build()?,
        })
    }

    pub async fn get_satellite_passes(
        &self,
        satellite_id: usize,
        location: &Location,
        days: usize,
        min_max_elevation: f64,
    ) -> anyhow::Result<SatellitePasses> {
        let url = format!(
            "https://api.n2yo.com/rest/v1/satellite/radiopasses/{}/{}/{}/{}/{}/{}&apiKey={}",
            satellite_id,
            location.latitude,
            location.longitude,
            location.altitude,
            days,
            min_max_elevation,
            self.api_key
        );

        info!("Sending request to {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<JsonSatellitePasses>()
            .await?;
        Ok(response.into())
    }

    pub async fn get_name_from_norad_id(&self, satellite_id: usize) -> anyhow::Result<String> {
        let url = format!(
            "https://api.n2yo.com/rest/v1/satellite/radiopasses/{}/{}/{}/{}/{}/{}&apiKey={}",
            satellite_id, 12.0, 12.0, 12.0, 12, 1, self.api_key
        );

        info!("Sending request to {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<JsonSatellitePasses>()
            .await?;
        Ok(response.info.name.clone())
    }
}

#[derive(Debug)]
pub struct SatellitePasses {
    pub info: SatellitePassInfo,
    pub passes: Vec<SatellitePass>,
}

impl From<JsonSatellitePasses> for SatellitePasses {
    fn from(json: JsonSatellitePasses) -> Self {
        Self {
            info: json.info,
            passes: json.passes.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonSatellitePasses {
    info: SatellitePassInfo,
    passes: Option<Vec<SatellitePass>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SatellitePassInfo {
    #[serde(rename = "satid")]
    pub id: usize,
    #[serde(rename = "satname")]
    pub name: String,
    #[serde(rename = "transactionscount")]
    pub transaction_count: usize,
    #[serde(rename = "passescount")]
    pub passes_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SatellitePass {
    #[serde(rename = "startAz")]
    pub start_azimuth: f64,
    #[serde(rename = "startAzCompass")]
    pub start_azimuth_compass: String,
    #[serde(rename = "startUTC")]
    pub start_utc: usize,
    #[serde(rename = "maxAz")]
    pub max_azimuth: f64,
    #[serde(rename = "maxAzCompass")]
    pub max_azimuth_compass: String,
    #[serde(rename = "maxEl")]
    pub max_elevation: f64,
    #[serde(rename = "maxUTC")]
    pub max_utc: usize,
    #[serde(rename = "endAz")]
    pub end_azimuth: f64,
    #[serde(rename = "endAzCompass")]
    pub end_azimuth_compass: String,
    #[serde(rename = "endUTC")]
    pub end_utc: usize,
}
