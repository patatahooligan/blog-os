# Blog-os

This is my implementation of Philipp Oppermann's Blog OS, a small
operating system written in Rust. The original material I'm following
can currently be found at https://os.phil-opp.com/ .

# Building

Currently, this requires some unstable features from the nightly branch
of Rust. If you are using `rustup`, you can install it with

    rustup install nightly

The project specifies that it requires the nightly branch, so you
shouldn't have to set it as your system default or anything.

Additionally, to build a bootable image we require the `bootimage` tool
and the `llvm-tools-preview` component.

    cargo install bootimage
    rustup component add llvm-tools-preview

Then, to create a bootable image, run

    cargo bootimage

# Running

You can (probably) write the bootable image to USB and boot into it. But
a more practical way to run this while testing/developing is to use
qemu. Make sure you have qemu installed for x86_64 and then simply run

    cargo run
