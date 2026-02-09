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
pub use cpu::flags;
pub use cpu::executor;