use crate::{
    command::Cargo,
    config::{self, BuildConfig, Info, RunInfo},
};
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn build(info: &Info) -> Result<RunInfo> {
    let cfg = handle_config(info)?;
    let user = build_user(info, &cfg.user)?;
    let kernel = build_kernel(info, &user)?;
    let efi_stub = build_stub(info, &kernel)?;
    build_efidir(info, &efi_stub)?;
    Ok(RunInfo {
        info,
        kernel,
        efi_stub,
    })
}

fn handle_config(info: &Info) -> Result<BuildConfig> {
    let file = if info.test() {
        "test.toml"
    } else {
        "build.toml"
    };
    let cfg: BuildConfig = config::parse(info, file)?;
    let out = info.out_dir();
    xshell::mkdir_p(&out)?;
    fs::write(out.clone().join("cfg_kernel.rs"), format!("{}", cfg.kernel))?;
    fs::write(out.join("cfg_uefi_stub.rs"), format!("{}", cfg.uefi_stub))?;
    Ok(cfg)
}

fn build_user(info: &Info, user: &str) -> Result<PathBuf> {
    println!("Building userspace...");
    Cargo::new("build")
        .with_info(info)
        .package(user)
        .env("RUST_TARGET_PATH", info.targetspec_dir())
        .target("x86_64-unknown-angstros")
        .z("build-std=core")
        .z("build-std-features=compiler-builtins-mem")
        .single_executable()
}

fn build_kernel(info: &Info, user: &Path) -> Result<PathBuf> {
    println!("Building kernel...");
    let mut cargo = Cargo::new(if info.test() { "test" } else { "build" });
    if info.test() {
        cargo.arg("--no-run");
    }
    cargo
        .with_info(info)
        .package("kernel")
        .env("RUST_TARGET_PATH", info.targetspec_dir())
        .target("x86_64-unknown-angstros")
        .z("build-std=core,alloc")
        .z("build-std-features=compiler-builtins-mem")
        .env("USER_PATH", user)
        .env("XTASK_OUT_DIR", info.out_dir())
        .single_executable()
}

fn build_stub(info: &Info, kernel: &Path) -> Result<PathBuf> {
    println!("Building UEFI stub...");
    Cargo::new("build")
        .with_info(info)
        .package("uefi_stub")
        .target("x86_64-unknown-uefi")
        .z("build-std=core")
        .z("build-std-features=compiler-builtins-mem")
        .env("KERNEL_PATH", kernel)
        .env("XTASK_OUT_DIR", info.out_dir())
        .single_executable()
}

fn build_efidir(info: &Info, stub: &Path) -> Result<()> {
    println!("Building EFI system partition...");
    let boot_dir = info.esp_dir().join("EFI/Boot");
    xshell::mkdir_p(&boot_dir)?;
    let efi_stub = boot_dir.join("BootX64.efi");
    xshell::cp(&stub, &efi_stub)?;
    Ok(())
}
