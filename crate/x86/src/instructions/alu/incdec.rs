use crate::cpu::X86Cpu;
use crate::flags;

use super::core::set_flags;

fn inc_width(cpu: &mut X86Cpu, reg: u8, bits32: bool) {
    let old_cf = cpu.registers.flags() & flags::CF;
    if bits32 {
        let old = cpu.read_reg32(reg);
        let new = old.wrapping_add(1);
        cpu.write_reg32(reg, new);
        let cf = old == 0xFFFF_FFFF;
        let of = old == 0x7FFF_FFFF;
        let af = (old & 0x0F) == 0x0F;
        let mut f = flags::compute_flags_u32(cpu.registers.flags(), new, cf, of, af);
        f = (f & !flags::CF) | old_cf;
        set_flags(cpu, f);
    } else {
        let old = cpu.read_reg16(reg);
        let new = old.wrapping_add(1);
        cpu.write_reg16(reg, new);
        let cf = old == 0xFFFF;
        let of = old == 0x7FFF;
        let af = (old & 0x0F) == 0x0F;
        let mut f = flags::compute_flags_u16(cpu.registers.flags(), new, cf, of, af);
        f = (f & !flags::CF) | old_cf;
        set_flags(cpu, f);
    }
}

fn dec_width(cpu: &mut X86Cpu, reg: u8, bits32: bool) {
    let old_cf = cpu.registers.flags() & flags::CF;
    if bits32 {
        let old = cpu.read_reg32(reg);
        let new = old.wrapping_sub(1);
        cpu.write_reg32(reg, new);
        let of = old == 0x8000_0000;
        let af = (old & 0x0F) == 0;
        let mut f = flags::compute_flags_u32(cpu.registers.flags(), new, false, of, af);
        f = (f & !flags::CF) | old_cf;
        set_flags(cpu, f);
    } else {
        let old = cpu.read_reg16(reg);
        let new = old.wrapping_sub(1);
        cpu.write_reg16(reg, new);
        let of = old == 0x8000;
        let af = (old & 0x0F) == 0;
        let mut f = flags::compute_flags_u16(cpu.registers.flags(), new, false, of, af);
        f = (f & !flags::CF) | old_cf;
        set_flags(cpu, f);
    }
}

pub fn inc_reg(cpu: &mut X86Cpu, reg: u8) {
    inc_width(cpu, reg, cpu.prefixes.has_operand_size);
}

pub fn dec_reg(cpu: &mut X86Cpu, reg: u8) {
    dec_width(cpu, reg, cpu.prefixes.has_operand_size);
}
