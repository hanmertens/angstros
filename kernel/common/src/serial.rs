//! Serial I/O port

use core::fmt::{Arguments, Write};
use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::interrupts;

static SERIAL1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3f8) });

/// Initialize serial devices. Should be called once before using any of the
/// print  functions and macros that use serial ports, including indirectly
/// (e.g. logging and panicking).
pub fn init() {
    SERIAL1.lock().init();
}

/// Print and format to the `SERIAL1` port. Beforehand [`init`] should be called.
pub fn print(args: Arguments) {
    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}

/// Format and print using [`print`] function.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::serial::print(format_args!($($arg)*));
    };
}

/// Format and print line using [`print`] function.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}
