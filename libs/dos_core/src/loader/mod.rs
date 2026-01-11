// dos_core/src/loader/mod.rs

mod mz;
mod com;

pub use mz::MzHeader;
pub use self::com::DosExecutable; // или как ты назвал основную структуру
