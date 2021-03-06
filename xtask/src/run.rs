use crate::{
    command::CommandResultExt,
    config::{self, Info, RunConfig, RunInfo},
};
use anyhow::{anyhow, Result};
use std::{
    io::ErrorKind,
    net::{Shutdown, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

pub fn debug(info: &RunInfo) -> Result<()> {
    let mut qemu = run_qemu(info.info, &["-s", "-S"])?;
    let gdb = run_gdb(&info.kernel);
    qemu.kill()?;
    gdb
}

pub fn run(info: &RunInfo) -> Result<()> {
    run_qemu(info.info, &[])?.wait().check_status("QEMU")
}

pub fn test(info: &RunInfo) -> Result<()> {
    let args = &["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"];
    run_qemu(info.info, args)?
        .wait()
        .map(|status| match status.code() {
            // This is the mangled kernel::test::ExitCode::Success
            Some(0x21) => Some(0),
            code => code,
        })
        .check_status("QEMU")
}

fn run_gdb(kernel: &Path) -> Result<()> {
    let mut max = 1000;
    let tick = 10;
    loop {
        match TcpStream::connect("127.0.0.1:1234") {
            Ok(c) => break c.shutdown(Shutdown::Both)?,
            Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                max -= 1;
                if max == 0 {
                    return Err(anyhow!("QEMU did not start within {}s", max * tick / 1000));
                }
                thread::sleep(Duration::from_millis(tick))
            }
            Err(e) => return Err(e.into()),
        }
    }
    println!("QEMU initialized; starting GDB...");
    Command::new("rust-gdb")
        .arg(kernel)
        .arg("-ex")
        .arg("target remote localhost:1234")
        .status()
        .check_status("GDB")
}

fn run_qemu(info: &Info, extra_args: &[&str]) -> Result<Child> {
    println!("Running kernel with QEMU...");
    let config: RunConfig = config::parse(info, "run.toml")?;
    Command::new("qemu-system-x86_64")
        .arg("-nodefaults")
        .args(config.qemu_args)
        .args(&["-serial", "stdio", "-vga", "std"])
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
        .args(extra_args)
        .stdin(Stdio::null())
        .spawn()
        .check_status("QEMU")
}
