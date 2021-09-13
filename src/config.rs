use std::path::PathBuf;

use eyre::Result;
use serde::Deserialize;
use serde::Serialize;
use tracing::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub port: u16,
    pub repositories: Vec<PathBuf>
}

impl Config {
    fn new_default() -> Self {
        Self {
            port: 9002,
            repositories: vec![],
        }
    }

    pub fn open_or_create() -> Result<Self> {
        let path = &crate::OPTIONS.config_file;

        trace!(?path, "Opening config file");

        if !path.is_file() {
            warn!(?path, "Config file not found. Creating a default one.");
            let file = std::fs::File::create(&path)?;
            serde_yaml::to_writer(file, &Self::new_default())?;
        }

        let config_file = std::fs::File::open(&path)?;

        let config: Self = serde_yaml::from_reader(config_file)?;
        if config.repositories.is_empty() {
            warn!(
                "The config file does not define any repositories.\
                This program will do nothing if no repositories are defined"
            );
        }
        Ok(config)
    }
}
