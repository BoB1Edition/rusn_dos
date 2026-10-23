use crate::cpu::X86Cpu;
use crate::flags;
use bus::Machine;

use super::core::set_flags;

pub fn daa(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let old_al = cpu.registers.al();
    let old_cf = flags::test_cf(cpu.registers.flags());
    let old_af = flags::test_af(cpu.registers.flags());
    let mut al = old_al;
    let mut cf = false;
    let mut af = false;
    if (old_al & 0x0F) > 9 || old_af {
        al = al.wrapping_add(6);
        af = true;
    }
    if al > 0x9F || old_cf {
        al = al.wrapping_add(0x60);
        cf = true;
    }
    cpu.registers.set_al(al);
    let of = flags::test_of(cpu.registers.flags());
    let f = flags::compute_flags_u8(cpu.registers.flags(), al, cf, of, af);
    set_flags(cpu, f);
}

pub fn das(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let old_al = cpu.registers.al();
    let old_cf = flags::test_cf(cpu.registers.flags());
    let old_af = flags::test_af(cpu.registers.flags());
    let mut al = old_al;
    let mut cf = false;
    let mut af = false;
    if (old_al & 0x0F) > 9 || old_af {
        al = al.wrapping_sub(6);
        af = true;
    }
    if al > 0x9F || old_cf {
        al = al.wrapping_sub(0x60);
        cf = true;
    }
    cpu.registers.set_al(al);
    let of = flags::test_of(cpu.registers.flags());
    let f = flags::compute_flags_u8(cpu.registers.flags(), al, cf, of, af);
    set_flags(cpu, f);
}
