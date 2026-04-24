// Ver: 1
pub use machine::DosMachine;
pub mod cpu;

pub mod error {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

mod registers;
mod memory;
mod machine;
mod consts;
mod modrm;
mod instructions;
mod interrupts;
pub mod loader;
pub mod filesystem;
pub mod video;
pub mod ivt;
pub use filesystem::DiskDriver;
pub(crate) use cpu::flags;
pub(crate) use cpu::executor;
pub(crate) use ivt::init_ivt;