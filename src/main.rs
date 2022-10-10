// For an operating system we won't have access to the std library or a
// runtime. Without a runtime, we also can't use the normal entry point
// of "main", see _start() below.
#![no_main]
#![no_std]

use core::panic::PanicInfo;

// Disable mangling because we need the linker to see this as exactly
// "_start". For similar reasons, use the C calling convention. The C
// linker looks for _start by default, so we don't have to explicitly
// state that this is the entry point.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

/// Custom panic handler. This is a requirement for no_std. We can't do
/// something truly meaningful at this time. Just loop forever, ie
/// freeze the system.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
