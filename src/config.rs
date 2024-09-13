use std::{fs::File, io::Read};

use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const PATH: &str = "./config.toml";

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub telegram: Telegram,
    pub bot: Bot,
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut toml_str = String::new();
        File::open(PATH).and_then(|mut file| file.read_to_string(&mut toml_str))?;

        Ok(toml::from_str::<Self>(&toml_str)?)
    }
}

#[derive(Deserialize, Serialize)]
pub struct Telegram {
    pub api_id: i32,
    pub api_hash: String,
}

#[derive(Deserialize, Serialize)]
pub struct Bot {
    pub token: String,
    pub catch_up: bool,
    pub flood_sleep_threshold: u32,
}
