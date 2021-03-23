use crate::{command::Cargo, config::BuildInfo};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn build(info: &BuildInfo) -> Result<()> {
    let kernel = build_kernel(info)?;
    let stub = build_stub(&kernel)?;
    build_efidir(info, &stub)?;
    Ok(())
}

fn build_kernel(info: &BuildInfo) -> Result<PathBuf> {
    println!("Building kernel...");
    Cargo::build()
        .package("kernel")
        .env("RUST_TARGET_PATH", info.targetspec_dir())
        .target("x86_64-unknown-angstros")
        .z("build-std=core")
        .z("build-std-features=compiler-builtins-mem")
        .single_executable()
}

fn build_stub(kernel: &Path) -> Result<PathBuf> {
    println!("Building UEFI stub...");
    Cargo::build()
        .package("uefi_stub")
        .target("x86_64-unknown-uefi")
        .z("build-std=core")
        .z("build-std-features=compiler-builtins-mem")
        .env("KERNEL_PATH", kernel)
        .single_executable()
}

fn build_efidir(info: &BuildInfo, stub: &Path) -> Result<()> {
    println!("Building EFI system partition...");
    let boot_dir = info.esp_dir().join("EFI/Boot");
    xshell::mkdir_p(&boot_dir)?;
    let efi_stub = boot_dir.join("BootX64.efi");
    xshell::cp(&stub, &efi_stub)?;
    Ok(())
}
