use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: String,
    pub frigate: Frigate,
}

#[derive(Debug, Deserialize)]
pub struct Frigate {
    pub url: String,
}

fn default_listen() -> String {
    "0.0.0.0:8080".to_string()
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(&path).context("reading config file")?;
        toml::from_str(&raw).context("parsing TOML config")
    }
}
