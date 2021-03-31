use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct BuildInfo {
    base_dir: PathBuf,
}

impl BuildInfo {
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> Self {
        let base_dir = base_dir.into();
        Self { base_dir }
    }

    pub fn targetspec_dir(&self) -> PathBuf {
        self.base_dir.join("data/targetspec")
    }

    pub fn esp_dir(&self) -> PathBuf {
        self.base_dir.join("target/esp")
    }

    pub fn config_dir(&self) -> PathBuf {
        self.base_dir.join("config")
    }
}

pub struct RunInfo<'a> {
    pub build_info: &'a BuildInfo,
    pub kernel: PathBuf,
    pub efi_stub: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RunConfig {
    pub ovmf_dir: PathBuf,
    pub qemu_args: Vec<String>,
}

/// Convenience method to deserialize struct directly from a file since the
/// `toml` create doesn't provide `from_reader`.
pub fn parse<P: AsRef<Path>, T: DeserializeOwned>(info: &BuildInfo, config: P) -> Result<T> {
    let config = config.as_ref();
    let context = || format!("Could not read config/{}", config.display());
    let bytes = fs::read(info.config_dir().join(config)).with_context(context)?;
    toml::from_slice(&bytes).with_context(context)
}
