use common::{print, println};
use core::panic::PanicInfo;
use owo_colors::OwoColorize;
use x86_64::instructions::port::Port;

/// Exit code to pass to QEMU
///
/// Note that these codes are "mangled" by QEMU: the exit code of QEMU will be
/// `(code << 1) | 0x1`
#[repr(u32)]
enum ExitCode {
    Success = 0x10,
    Failure = 0x11,
}

/// Write exit code to port 0xf4
///
/// QEMU can be configured to shut down this way with
/// `-device isa-debug-exit,iobase=0xf4,iosize=0x04`
///
/// # Safety
/// Port should exist (the case if QEMU is used)
fn exit(exit_code: ExitCode) {
    let mut port = Port::<u32>::new(0xf4);
    unsafe { port.write(exit_code as u32) };
}

pub fn test_runner(tests: &[&dyn Test]) {
    println!();
    println!(
        "running {} test{}",
        tests.len(),
        if tests.len() == 1 { "" } else { "s" }
    );

    for test in tests {
        test.run();
    }

    println!();
    println!(
        "test result: {}. {} passed; 0 failed",
        "ok".green(),
        tests.len()
    );
    println!();

    exit(ExitCode::Success);
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}\n", "failed".red());
    log::error!("{:#?}", info);
    exit(ExitCode::Failure);
    common::panic_handler(info);
}

pub trait Test {
    fn run(&self);
}

impl<F: Fn()> Test for F {
    fn run(&self) {
        print!("test {} ... ", core::any::type_name::<F>());
        self();
        println!("{}", "ok".green());
    }
}
