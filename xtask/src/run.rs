use crate::{
    command::CommandResultExt,
    config::{self, BuildInfo, RunConfig},
};
use anyhow::Result;
use std::process::Command;

pub fn run(info: &BuildInfo) -> Result<()> {
    run_qemu(info)
}

fn run_qemu(info: &BuildInfo) -> Result<()> {
    println!("Running kernel with QEMU...");
    let config: RunConfig = config::parse(info, "run.toml")?;
    Command::new("qemu-system-x86_64")
        .arg("-nodefaults")
        .args(config.qemu_args)
        .args(&["-serial", "stdio"])
        .arg("-drive")
        .arg(format!(
            "if=pflash,format=raw,file={},readonly",
            config.ovmf_dir.join("OVMF_CODE.fd").display()
        ))
        .arg("-drive")
        .arg(format!(
            "if=pflash,format=raw,file={},readonly",
            config.ovmf_dir.join("OVMF_VARS.fd").display()
        ))
        .arg("-drive")
        .arg(format!(
            "format=raw,file=fat:rw:{}",
            info.esp_dir().display()
        ))
        .status()
        .check_status("QEMU")
}
