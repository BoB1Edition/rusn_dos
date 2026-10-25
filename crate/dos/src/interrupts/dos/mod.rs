// Ver: 1 File: ./crate/dos/src/interrupts/dos/mod.rs
mod dispatch;
mod console;
mod file_ops;
mod dir;
mod memory;
mod find;
mod misc;

pub use dispatch::{handle_int21, handle_int2f};
