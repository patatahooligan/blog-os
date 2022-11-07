//! Print text to serial port
//!
//! This is intended to be used with qemu. The serial is redirected by
//! qemu to the stdout of the host terminal. So you can use
//! [crate::serial_print] and [crate::serial_println] to print messages
//! on the host. But you could use it on another serial device if you
//! want.

use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

/// Print to the host through serial interface, analogous to
/// `std::fmt::print`.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) =>  ($crate::serial::_print(format_args!($($arg)*)));
}

/// Print to the host through serial interface, analogous to
/// `std::fmt::println`. Equivalent to [serial_print] with an appended
/// `\n`.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}

// This is not really intended to be a part of the public API, but it
// has to be since the macros use it. Let's at least hide it in the
// docs.
#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}
