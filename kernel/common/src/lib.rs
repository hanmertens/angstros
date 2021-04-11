//! Boot code shared between different crates (e.g. the UEFI stub and the
//! kernel).

#![no_std]

pub mod boot;
pub mod elf;
pub mod logger;
pub mod serial;

use core::panic::PanicInfo;
use log::LevelFilter;
use owo_colors::OwoColorize;
use x86_64::instructions;

/// Initialize all relevant structures before use
///
/// Initializes the serial port and logger.
pub fn init(log_filter: LevelFilter) -> Result<(), &'static str> {
    serial::init();
    logger::init(log_filter).map_err(|_| "Could not initialize logger")?;
    Ok(())
}

/// Print the panic information via SERIAL1 and halt the CPU indefinitely.
pub fn panic_handler(info: &PanicInfo) -> ! {
    println!();
    println!(
        "{}",
        "KERNEL PANIC -- An unrecoverable error has occurred!".on_red()
    );
    println!();
    println!("{:#?}", info);
    loop {
        instructions::hlt();
    }
}
