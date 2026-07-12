// Ver: 1 File: crate/bus/src/cpu.rs
use crate::Machine;

/// Базовый трейт для любого процессора.
/// Позволяет материнской плате запускать и опрашивать состояние CPU.
pub trait Cpu {
    /// Выполняет ровно одну инструкцию (или один цикл префиксов + инструкцию).
    fn step(&mut self, machine: &mut dyn Machine);
    
    /// Находится ли процессор в состоянии остановки (HLT).
    fn is_halted(&self) -> bool;
    
    /// Принудительно остановить процессор.
    fn halt(&mut self);
}