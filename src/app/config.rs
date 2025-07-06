use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub city: String,
    pub country: String,
    pub method: u8,
    pub madhab: u8,
}

fn get_config_dir() -> PathBuf {
    let mut path = env::var("HOME").unwrap();
    path.push_str("/.config/prayer-tui");
    PathBuf::from(path)
}

pub fn load_config() -> Result<Config, io::Error> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    let path = config_dir.join("config.toml");
    if !path.exists() {
        let default_config = Config {
            city: "Seattle".to_string(),
            country: "US".to_string(),
            method: 2,
            madhab: 1,
        };
        let toml = toml::to_string(&default_config).unwrap();
        let mut file = File::create(&path)?;
        file.write_all(toml.as_bytes())?;
        return Ok(default_config);
    }

    let mut file = File::open(&path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Config = toml::from_str(&contents).unwrap();
    Ok(config)
}

