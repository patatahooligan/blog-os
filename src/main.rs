// For an operating system we won't have access to the std library or a
// runtime. Without a runtime, we also can't use the normal entry point
// of "main", see _start() below.
#![no_main]
#![no_std]
// Because of #![no_std], we don't have access to the normal testing
// framework (and we probably couldn't use it anyway because of the
// peculiar nature of our executable). So we define our own test runner.
// Additionally, Rust would create a `main` function for the test
// executable, but since we use `_start` instead of `main`, we call
// the test runner from `_start`.
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod serial;
mod vga_buffer;

use core::panic::PanicInfo;

/// Print "Hello World!" using the VGA buffer. This is a toy _start
/// function to simply have something to test.
// Disable mangling because we need the linker to see this as exactly
// "_start". For similar reasons, use the C calling convention. The C
// linker looks for _start by default, so we don't have to explicitly
// state that this is the entry point.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello {}!", "world");

    #[cfg(test)]
    test_main();

    // Since our executable is an OS, it can't simply exit. Looping
    // indefinitely is a way to "stop" when we're done.
    loop {}
}

/// Custom panic handler. This is a requirement for no_std. We can't do
/// something truly meaningful at this time. Just loop forever, ie
/// freeze the system.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

/// Panic handler for tests. Unlike the normal panic handler, this one
/// prints to the serial port that is redirected to stdout.
/// Additionally, it closes qemu and returns an exit code to indicate
/// failure.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

/// An exit code that can be passed to qemu's serial port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Exit qemu with the given exit code. Note that qemu shifts this value
/// to add a trailing 1 bit. The result is (exit_code << 1) | 1.
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }

    exit_qemu(QemuExitCode::Success);
}

/// Silly assertion just to make sure the testing framework is working.
/// Because of our peculiar setup that is not a given.
#[test_case]
fn trivial_assertion() {
    serial_print!("Trivial assertion... ");
    assert_eq!(1, 1);
    serial_println!("[ok]");
}
