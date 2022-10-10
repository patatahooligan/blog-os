// For an operating system we won't have access to the std library or a
// runtime. Without a runtime, we also can't use the normal entry point
// of "main", see _start() below.
#![no_main]
#![no_std]

use core::panic::PanicInfo;

static HELLO: &[u8] = b"Hello World!";

/// Print "Hello World!" using the VGA buffer. This is a toy _start
/// function to simply have something to test.
// Disable mangling because we need the linker to see this as exactly
// "_start". For similar reasons, use the C calling convention. The C
// linker looks for _start by default, so we don't have to explicitly
// state that this is the entry point.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // The VGA standard guarantees that we have a VGA buffer at address
    // 0xb8000 that is a 80x25 array of (ASCII code, color) byte tuples.
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        // TODO: move the unsafe part to some wrapper class.
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    // Since our executable is an OS, it can't simply exit. Looping
    // indefinitely is a way to "stop" when we're done.
    loop {}
}

/// Custom panic handler. This is a requirement for no_std. We can't do
/// something truly meaningful at this time. Just loop forever, ie
/// freeze the system.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
