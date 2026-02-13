// Ver: 2
pub use machine::DosMachine;
pub mod cpu;

pub mod error {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

mod registers;
mod memory;
mod machine;
mod dos_api;
mod consts;
mod modrm;
mod instructions;
mod interrupts;
pub mod loader;
pub mod filesystem;
pub mod video;

pub use filesystem::DiskDriver;
pub use cpu::flags;
pub use cpu::executor;