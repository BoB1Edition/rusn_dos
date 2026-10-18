// Ver: 1 File: crate/dos/src/lib.rs

pub mod consts;
pub mod filesystem;
pub mod interrupts;
pub mod ivt;
pub mod loader;
pub mod mcb;
pub mod system;

pub mod error {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

pub use bus::Display;
pub use filesystem::{DiskDriver, FileSystem};
pub use system::{DosSystem, EmsState};
