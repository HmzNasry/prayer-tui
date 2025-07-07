use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{File},
    io::{self, Read, Write},
    path::PathBuf,
};

use chrono::Local;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppState {
    pub notified_prayers: Vec<String>,
    pub date: String,
}

fn get_config_dir() -> PathBuf {
    let mut path = env::var("HOME").unwrap();
    path.push_str("/.config/prayer-tui");
    PathBuf::from(path)
}

pub fn load_app_state() -> Result<AppState, io::Error> {
    let config_dir = get_config_dir();
    let path = config_dir.join("state.json");
    if !path.exists() {
        return Ok(AppState {
            notified_prayers: Vec::new(),
            date: Local::now().format("%Y-%m-%d").to_string(),
        });
    }

    let mut file = File::open(&path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let state: AppState = serde_json::from_str(&contents)?;
    Ok(state)
}

pub fn save_app_state(state: &AppState) -> Result<(), io::Error> {
    let config_dir = get_config_dir();
    let path = config_dir.join("state.json");
    let json = serde_json::to_string(state)?;
    let mut file = File::create(&path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

