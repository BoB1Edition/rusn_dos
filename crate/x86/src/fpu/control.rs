// Ver: 1 File: crate/x86/src/fpu/control.rs
//! Инструкции управления FPU: FINIT/FNINIT, FLDCW/FNSTCW, FNSTSW.
//!
//! Wait/no-wait пары (FINIT/FNINIT, FSTCW/FNSTCW, FSTSW/FNSTSW) на P6+
//! идентичны по поведению (нет асинхронной очереди FPU), поэтому реализация
//! общая. Кодировки различаются, и esc.rs вызывает нужную.

use bus::Machine;

use crate::cpu::X86Cpu;

/// FNINIT (D9 E0) / FINIT (DB E0).
///
/// Сброс FPU: все регистры пусты, TOP=0, status=0, control=0x037F.
pub fn fninit(cpu: &mut X86Cpu) {
    cpu.fpu.finit();
}

/// FINIT — wait-вариант, та же реализация.
pub fn finit(cpu: &mut X86Cpu) {
    cpu.fpu.finit();
}

/// FLDCW m16 (D9 /5). Загрузка контрольного слова из памяти.
pub fn fldcw(cpu: &mut X86Cpu, machine: &mut dyn Machine, addr: u32) {
    cpu.fpu.control = machine.read_mem_u16(addr);
}

/// FNSTCW m16 (D9 /7). Сохранение контрольного слова в память.
pub fn fnstcw(cpu: &mut X86Cpu, machine: &mut dyn Machine, addr: u32) {
    machine.write_mem_u16(addr, cpu.fpu.control);
}

/// FNSTSW AX (DF E0). Сохранение слова статуса в AX.
pub fn fnstsw_ax(cpu: &mut X86Cpu) {
    cpu.registers.set_ax(cpu.fpu.status);
}

/// FNSTSW m16 (DF /7). Сохранение слова статуса в память.
pub fn fnstsw_m16(cpu: &mut X86Cpu, machine: &mut dyn Machine, addr: u32) {
    machine.write_mem_u16(addr, cpu.fpu.status);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpu::state::CTRL_DEFAULT;
    use bus::Motherboard;

    #[test]
    fn test_fninit_resets() {
        let mut cpu = X86Cpu::new();
        cpu.fpu.status = 0xFFFF;
        cpu.fpu.control = 0;
        fninit(&mut cpu);
        assert_eq!(cpu.fpu.status, 0);
        assert_eq!(cpu.fpu.control, CTRL_DEFAULT);
        assert_eq!(cpu.fpu.top(), 0);
    }

    #[test]
    fn test_fldcw_fnstcw_roundtrip() {
        let mut cpu = X86Cpu::new();
        let mut mb = Motherboard::new(64);
        mb.write_mem_u16(0x10, 0x1234);
        fldcw(&mut cpu, &mut mb, 0x10);
        assert_eq!(cpu.fpu.control, 0x1234);
        fnstcw(&mut cpu, &mut mb, 0x20);
        assert_eq!(mb.read_mem_u16(0x20), 0x1234);
    }

    #[test]
    fn test_fnstsw_ax() {
        let mut cpu = X86Cpu::new();
        cpu.fpu.status = 0xABCD;
        fnstsw_ax(&mut cpu);
        assert_eq!(cpu.registers.ax(), 0xABCD);
    }

    #[test]
    fn test_fnstsw_m16() {
        let mut cpu = X86Cpu::new();
        let mut mb = Motherboard::new(64);
        cpu.fpu.status = 0x1234;
        fnstsw_m16(&mut cpu, &mut mb, 0x30);
        assert_eq!(mb.read_mem_u16(0x30), 0x1234);
    }
}