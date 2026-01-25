pub use loader::DosExecutable;
pub use machine::DosMachine;

pub mod error {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

mod registers;
mod memory;
mod machine;
mod dos_api;
mod consts;
mod modrm;

pub mod loader;
mod instructions;