//! Write text to the VGA buffer
//!
//! The simplest way to use this crate is via [crate::print] and
//! [crate::println]. Note that we don't have access to `std::print` and
//! `std::println`. So it is recommended to `use` these macros so that
//! you can use them in place of the `std` ones like a normal Rust
//! program would.
//!
//! ```
//! use blog_os::println;
//!
//! fn kernel_main(_boot_info: &'static BootInfo) -> ! {
//!     println!("Hello {}!", "world");
//! }
//! ```
//!
//! You can use the [struct@WRITER] directly, but it's not recommended
//! as it doesn't offer anything over the macros. Note that
//! [struct@WRITER] already holds a mutable reference to the VGA buffer
//! (`0xb8000`), so don't create another [Writer] instance for the same
//! buffer!

use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// Print to the vga buffer, similar to how `std::fmt::print` would
/// behave in a terminal if it were available to us.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Print to the vga buffer, similar to how `std::fmt::println` would
/// behave in a terminal if it were available to us. Equivalent to
/// [print] with an appended `\n`.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

// This is not really intended to be a part of the public API, but it
// has to be since the macros use it. Let's at least hide it in the
// docs.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

// The lazy static is required here because we don't want compile time
// evaluation of the pointer.
//
// TODO: Revaluate the need for lazy_static. Some of Rust's restrictions
// on statics have been relaxed. Additionally, there might be better
// libraries for it.
lazy_static! {
    /// Global static instance of [Writer]. Has a simple spinlock to
    /// allow use in multithreaded kernels.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

/// Color byte for the VGA buffer. The VGA buffer requires both a
/// foreground and a background color, so we can't use this enum
/// directly. Use [ColorCode] as the VGA color byte instead.
// Rust will consider dead code every single Color we do not use. Since
// we don't intend to use every one, we supress warnings for this enum
// in particular.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// A color code containing both a foreground and a background color.
/// The information is encoded in a single byte as per the VGA buffer.
/// You can definitely write this directly into the VGA buffer, but
/// most of the time you want to create a [ScreenChar] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A tuple of (ASCII code, color code) that represents a single
/// character on the screen as per the VGA buffer standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// The entire VGA buffer. It is a 2d array of [ScreenChar]s. For
/// safety reasons, this should be manipulated through a [Writer].
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// Write to the VGA buffer. This works like a character stream, where
/// the user is not required or allowed to manipulate the buffer
/// directly. Instead [Writer] keeps track of where the cursor is and
/// makes sure to keep everything in bounds. Specifically it makes sure
/// to scroll the screen if it would overflow.
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Write a single byte to the screen. To change lines, pass a '\n'
    /// character.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                // If the line is full, move to the next one
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });

                self.column_position += 1;
            }
        }
    }

    /// Convenience function to call [Writer::write_byte] on every byte
    /// of a string.
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            // str is UTF-8 but the VGA buffer supports CCSID 437 only.
            // We can deal with this by transforming unprintable
            // characters to a printable placeholder.
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// Move to new line, essentially do what you expect for '\n'.
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Some test string that fits on a single line";
    // Disable interrupts and keep the writer locked for the entire
    // duration of the test. Otherwise we might have output from
    // another thread interfere with the test.
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();

        // Note the leading '\n' in the output. Make sure we change
        // lines before printing the string. Later we want to be able to
        // compare the last printed line with the intended output so
        // there must be no leading characters from a previous print.
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}
