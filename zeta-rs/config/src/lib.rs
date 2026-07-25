//! Atomically persisted ordinary configuration. Secrets are intentionally excluded.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub struct ConfigLocation(pub PathBuf);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub preferred_model: Option<String>,
    pub theme: Option<Theme>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

pub struct ConfigStore {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl ConfigStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ConfigError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        Ok(Self {
            path,
            write_lock: Mutex::new(()),
        })
    }
    pub fn read(&self) -> Result<Config, ConfigError> {
        if !self.path.exists() {
            return Ok(Config::default());
        }
        serde_json::from_slice(&fs::read(&self.path).map_err(io_error)?)
            .map_err(|error| ConfigError(error.to_string()))
    }
    pub fn update(&self, update: ConfigUpdate) -> Result<Config, ConfigError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| ConfigError("config write lock poisoned".into()))?;
        let mut config = self.read()?;
        if let Some(model) = update.preferred_model {
            config.preferred_model = model;
        }
        if let Some(theme) = update.theme {
            config.theme = theme;
        }
        self.write_atomic(&config)?;
        Ok(config)
    }
    fn write_atomic(&self, config: &Config) -> Result<(), ConfigError> {
        let temporary = self.path.with_extension("tmp");
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(config).map_err(|error| ConfigError(error.to_string()))?,
        )
        .map_err(io_error)?;
        fs::rename(temporary, &self.path).map_err(io_error)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConfigUpdate {
    pub preferred_model: Option<Option<String>>,
    pub theme: Option<Option<Theme>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigError(pub String);
fn io_error(error: impl std::fmt::Display) -> ConfigError {
    ConfigError(error.to_string())
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
