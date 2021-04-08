use crate::{
    command::Cargo,
    config::{BuildInfo, RunInfo},
};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn build(info: &BuildInfo, test: bool) -> Result<RunInfo> {
    let kernel = build_kernel(info, test)?;
    let efi_stub = build_stub(&kernel)?;
    build_efidir(info, &efi_stub)?;
    Ok(RunInfo {
        build_info: info,
        kernel,
        efi_stub,
    })
}

fn build_kernel(info: &BuildInfo, test: bool) -> Result<PathBuf> {
    println!("Building kernel...");
    let mut cargo = Cargo::new(if test { "test" } else { "build" });
    if test {
        cargo.arg("--no-run");
    }
    cargo
        .package("kernel")
        .env("RUST_TARGET_PATH", info.targetspec_dir())
        .target("x86_64-unknown-angstros")
        .z("build-std=core,alloc")
        .z("build-std-features=compiler-builtins-mem")
        .single_executable()
}

fn build_stub(kernel: &Path) -> Result<PathBuf> {
    println!("Building UEFI stub...");
    Cargo::new("build")
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
