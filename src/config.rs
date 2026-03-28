use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::data::VerseCollection;

const APP_NAME: &str = "bible-verse-memory";
const DATA_FILENAME: &str = "verses.yaml";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    pub data_path: Option<String>,
}

pub struct Config {
    pub data_path: PathBuf,
    config_file_path: PathBuf,
    data_path_overridden: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get config directory"))?;
        let config_file_yaml = config_dir.join(format!("{APP_NAME}.yaml"));
        let config_file_yml = config_dir.join(format!("{APP_NAME}.yml"));

        let config_file: ConfigFile = if config_file_yaml.exists() {
            let content = fs::read_to_string(&config_file_yaml)?;
            serde_yaml::from_str(&content).unwrap_or_default()
        } else if config_file_yml.exists() {
            let content = fs::read_to_string(&config_file_yml)?;
            serde_yaml::from_str(&content).unwrap_or_default()
        } else {
            let default_config = ConfigFile::default();
            if let Some(parent) = config_file_yaml.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_yaml::to_string(&default_config)?;
            fs::write(&config_file_yaml, content)?;
            default_config
        };

        let mut data_path = if let Some(configured_path) = &config_file.data_path {
            if let Some(stripped) = configured_path.strip_prefix("~/") {
                let home = dirs::home_dir()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get home directory"))?;
                home.join(stripped)
            } else if configured_path == "~" {
                dirs::home_dir()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get home directory"))?
            } else {
                let path = PathBuf::from(configured_path);
                if path.is_absolute() {
                    path
                } else {
                    config_dir.join(configured_path)
                }
            }
        } else if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DATA_FILENAME)
        } else {
            let data_dir = dirs::data_dir()
                .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get data directory"))?;
            data_dir.join(APP_NAME).join(DATA_FILENAME)
        };

        let data_path_overridden = cfg!(debug_assertions);
        if data_path_overridden {
            data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DATA_FILENAME);
        }

        let config_file_path = if config_file_yaml.exists() {
            config_file_yaml
        } else if config_file_yml.exists() {
            config_file_yml
        } else {
            config_file_yaml
        };

        Ok(Self {
            data_path,
            config_file_path,
            data_path_overridden,
        })
    }

    pub fn config_file_path(&self) -> &PathBuf {
        &self.config_file_path
    }

    pub fn data_path_overridden(&self) -> bool {
        self.data_path_overridden
    }

    pub fn data_path_absolute(&self) -> PathBuf {
        if self.data_path.is_absolute() {
            self.data_path.clone()
        } else {
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let absolute = current_dir.join(&self.data_path);
            absolute.canonicalize().unwrap_or(absolute)
        }
    }
}

pub fn load_verses(config: &Config) -> Result<VerseCollection> {
    if !config.data_path.exists() {
        return Ok(VerseCollection::new());
    }
    let content = fs::read_to_string(&config.data_path)?;
    let collection: VerseCollection = serde_yaml::from_str(&content)?;
    Ok(collection)
}

pub fn save_verses(collection: &VerseCollection, config: &Config) -> Result<()> {
    if let Some(parent) = config.data_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_yaml::to_string(collection)?;
    fs::write(&config.data_path, content)?;
    Ok(())
}
