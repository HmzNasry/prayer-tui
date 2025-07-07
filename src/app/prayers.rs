use reqwest::Error;
use serde::Deserialize;

use crate::app::config::Config;

#[derive(Deserialize, Debug)]
struct Data {
    data: Timings,
}

#[derive(Deserialize, Debug)]
struct Timings {
    timings: PrayerTimes,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct PrayerTimes {
    #[serde(rename = "Fajr")]
    pub fajr: String,
    #[serde(rename = "Dhuhr")]
    pub dhuhr: String,
    #[serde(rename = "Asr")]
    pub asr: String,
    #[serde(rename = "Maghrib")]
    pub maghrib: String,
    #[serde(rename = "Isha")]
    pub isha: String,
    #[serde(rename = "Sunrise")]
    pub sunrise: String,
    #[serde(rename = "Sunset")]
    pub sunset: String,
}

pub async fn get_prayer_times(config: &Config) -> Result<PrayerTimes, Error> {
    let url = format!(
        "https://api.aladhan.com/v1/timingsByCity?city={}&country={}&method={}&madhab={}",
        config.city,
        config.country,
        config.method,
        config.madhab
    );
    let response = reqwest::get(&url).await?.json::<Data>().await?;
    Ok(response.data.timings)
}

