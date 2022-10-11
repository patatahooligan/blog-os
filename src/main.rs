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
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use blog_os::println;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    println!("Hello {}!", "world");

    blog_os::init();

    #[cfg(test)]
    test_main();

    // Since our executable is an OS, it can't simply exit. Looping
    // indefinitely is a way to "stop" when we're done.
    blog_os::hlt_loop();
}

/// Custom panic handler. This is a requirement for no_std. We can't do
/// something truly meaningful at this time. Just loop forever, ie
/// freeze the system.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    blog_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

/// Silly assertion just to make sure the testing framework is working.
/// Because of our peculiar setup that is not a given.
#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
