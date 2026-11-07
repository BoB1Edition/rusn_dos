// Ver: 2 File: ./crate/x86/src/instructions/alu/bcd.rs
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
    let f = flags::compute_flags_u8(cpu.registers.flags(), al, cf, of, af, false);
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
    let f = flags::compute_flags_u8(cpu.registers.flags(), al, cf, of, af, false);
    set_flags(cpu, f);
}

/// 37h — AAA (ASCII Adjust AL after Addition)
pub fn aaa(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let al = cpu.registers.al();
    let ah = cpu.registers.ah();
    let af_in = flags::test_af(cpu.registers.flags());

    let (new_al, new_ah, adjust) = if (al & 0x0F) > 9 || af_in {
        (al.wrapping_add(6), ah.wrapping_add(1), true)
    } else {
        (al, ah, false)
    };

    cpu.registers.set_al(new_al & 0x0F);
    cpu.registers.set_ah(new_ah);

    let mut f = cpu.registers.flags();
    if adjust {
        f |= flags::AF | flags::CF;
    } else {
        f &= !(flags::AF | flags::CF);
    }
    cpu.registers.set_flags(f);
}

/// 3Fh — AAS (ASCII Adjust AL after Subtraction)
pub fn aas(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let al = cpu.registers.al();
    let ah = cpu.registers.ah();
    let af_in = flags::test_af(cpu.registers.flags());

    let (new_al, new_ah, adjust) = if (al & 0x0F) > 9 || af_in {
        (al.wrapping_sub(6), ah.wrapping_sub(1), true)
    } else {
        (al, ah, false)
    };

    cpu.registers.set_al(new_al & 0x0F);
    cpu.registers.set_ah(new_ah);

    let mut f = cpu.registers.flags();
    if adjust {
        f |= flags::AF | flags::CF;
    } else {
        f &= !(flags::AF | flags::CF);
    }
    cpu.registers.set_flags(f);
}