#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod serial;
pub mod vga_buffer;

#[cfg(test)]
use bootloader::{entry_point, BootInfo};
pub use core::panic::PanicInfo;

/// Initialize all structures required by the kernel.
pub fn init() {
    interrupts::init_idt();
    gdt::init();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

/// Loop endlessly, calling `hlt` on every iteration. This should be
/// used in every place where we want an empty infinite loop to keep the
/// kernel running and reacting to interrupts, but we don't really have
/// anything to do in the current thread.
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}

#[cfg(test)]
entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    hlt_loop()
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop()
}

/// Panic handler for tests. Unlike the normal panic handler, this one
/// prints to the serial port that is redirected to stdout.
/// Additionally, it closes qemu and returns an exit code to indicate
/// failure.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

/// An exit code that can be passed to qemu's serial port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Wrapper type to use for unit tests.
pub trait Testable {
    /// Simple wrapper to eliminate unit test boilerplate.
    ///  - print object's name
    ///  - run `self`
    ///  - print "\[ok]\"
    ///
    /// This never prints "\[failed\]" or similar, because if a test
    /// fails, the panic handler does that.
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
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

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }

    exit_qemu(QemuExitCode::Success);
}

/// Throw a breakpoint exception to verify that it works. Note that this
/// does not check its behavior. But the fact that the function returns
/// instead of panicking at leats verifies that we register the
/// exception handler.
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
