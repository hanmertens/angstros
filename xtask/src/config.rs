use anyhow::{Context, Result};
use clap::Clap;
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

/// Determine base directory of workspace based on xtask manifest
fn default_base_dir() -> &'static str {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.ancestors().nth(1).unwrap().to_str().unwrap()
}

#[derive(Clap)]
pub struct Info {
    /// Path to base directory of workspace
    #[clap(long, parse(from_os_str), default_value = default_base_dir())]
    base_dir: PathBuf,
    /// Path to directory containing configuration files
    #[clap(long, parse(from_os_str))]
    config_dir: Option<PathBuf>,
    /// Build in release mode with optimizations
    #[clap(long)]
    pub release: bool,
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl Info {
    pub fn test(&self) -> bool {
        self.cmd == SubCommand::Test
    }

    pub fn targetspec_dir(&self) -> PathBuf {
        self.base_dir.join("data/targetspec")
    }

    pub fn out_dir(&self) -> PathBuf {
        self.base_dir.join("target/xtask/out")
    }

    pub fn esp_dir(&self) -> PathBuf {
        self.base_dir.join("target/xtask/esp")
    }

    pub fn config_dir(&self) -> PathBuf {
        self.config_dir
            .clone()
            .unwrap_or_else(|| self.base_dir.join("config"))
    }
}

#[derive(Clap, PartialEq)]
pub enum SubCommand {
    /// Build kernel
    Build,
    /// Run kernel in QEMU and attach GDB as debugger
    Debug,
    /// Run kernel in QEMU
    Run,
    /// Run kernel tests in QEMU
    Test,
}

pub struct RunInfo<'a> {
    pub info: &'a Info,
    pub kernel: PathBuf,
    pub efi_stub: PathBuf,
}

fn camel_case(s: &str) -> String {
    s.split(' ')
        .map(|s| {
            let (a, b) = s.split_at(1);
            a.to_uppercase() + b
        })
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildConfig {
    pub user: String,
    pub uefi_stub: StubConfig,
    pub kernel: KernelConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct StubConfig {
    log_level: String,
}

impl fmt::Display for StubConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "pub const LOG_LEVEL: log::LevelFilter = log::LevelFilter::{};",
            camel_case(&self.log_level)
        )?;
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct KernelConfig {
    log_level: String,
    allocator: String,
}

impl fmt::Display for KernelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "pub const LOG_LEVEL: log::LevelFilter = log::LevelFilter::{};",
            camel_case(&self.log_level)
        )?;
        writeln!(
            f,
            "pub type Allocator = crate::allocator::{}Allocator;",
            camel_case(&self.allocator)
        )?;
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RunConfig {
    pub ovmf_dir: PathBuf,
    pub qemu_args: Vec<String>,
}

/// Convenience method to deserialize struct directly from a file since the
/// `toml` create doesn't provide `from_reader`.
pub fn parse<P: AsRef<Path>, T: DeserializeOwned>(info: &Info, config: P) -> Result<T> {
    let config = info.config_dir().join(config.as_ref());
    let context = || format!("Could not read {}", config.display());
    let bytes = fs::read(&config).with_context(context)?;
    toml::from_slice(&bytes).with_context(context)
}
