// Ver: 1 File: ./libs/dos_core/src/instructions/mod.rs
pub mod stack;
pub(crate) mod alu32;
pub(crate) mod incs;
pub(crate) mod alu;
pub(crate) mod bcd;
pub(crate) mod exchange;
pub(crate) mod segment;
pub mod mov32;
pub mod mov;
pub mod control;
pub mod control32;
pub mod system;
pub mod extended;
pub mod extended32;