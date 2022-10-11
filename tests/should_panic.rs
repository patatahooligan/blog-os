#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]

use blog_os::{exit_qemu, serial_print, serial_println, QemuExitCode};
use core::panic::PanicInfo;

/// Custom panic handler that returns a success exit code. This is
/// required to test our panic mechanism
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);

    loop {}
}

/// Make a trivial false assertion. This tests two different but related
/// aspects:
///  - We have managed to set a custom panic handler
///  - Our testing framework works. Our tests can panic thanks to the
///    above point, and the panic handler can close qemu and return an
///    exit code.
fn should_fail() {
    serial_print!("should_panic::should_fail...\t");
    assert_eq!(0, 1);
}
