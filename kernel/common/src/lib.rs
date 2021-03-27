//! Boot code shared between different crates (e.g. the UEFI stub and the
//! kernel).

#![no_std]

pub mod boot;
pub mod serial;

/// Initialize all relevant structures before use
///
/// Currently only initializes the serial port.
pub fn init() {
    serial::init();
}
