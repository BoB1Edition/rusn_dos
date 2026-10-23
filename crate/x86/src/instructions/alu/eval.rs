use crate::cpu::X86Cpu;
use crate::modrm::ModRm;
use bus::Machine;

use super::core::{add8, add16, add32, adc8, adc16, adc32, sub8, sub16, sub32, sbb8, sbb16, sbb32};
use super::core::{apply_arith8, apply_arith16, apply_arith32, apply_logic8, apply_logic16, apply_logic32, carry, Op, Width};
use super::util::{read_rm8, read_rm16, read_rm32, write_rm8, write_rm16, write_rm32};

pub fn binop_rm_r(cpu: &mut X86Cpu, machine: &mut dyn Machine, width: Width, op: Op) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    match width {
        Width::B8 => {
            let src = cpu.read_reg8(modrm.reg_field);
            let (dst, addr) = read_rm8(cpu, machine, modrm);
            let (result, write) = eval8(cpu, op, dst, src);
            if write {
                write_rm8(cpu, machine, modrm, addr, result);
            }
        }
        Width::B16 => {
            let src = cpu.read_reg16(modrm.reg_field);
            let (dst, addr) = read_rm16(cpu, machine, modrm);
            let (result, write) = eval16(cpu, op, dst, src);
            if write {
                write_rm16(cpu, machine, modrm, addr, result);
            }
        }
        Width::B32 => {
            let src = cpu.read_reg32(modrm.reg_field);
            let (dst, addr) = read_rm32(cpu, machine, modrm);
            let (result, write) = eval32(cpu, op, dst, src);
            if write {
                write_rm32(cpu, machine, modrm, addr, result);
            }
        }
    }
}

pub fn binop_r_rm(cpu: &mut X86Cpu, machine: &mut dyn Machine, width: Width, op: Op) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    match width {
        Width::B8 => {
            let (src, _) = read_rm8(cpu, machine, modrm);
            let dst = cpu.read_reg8(modrm.reg_field);
            let (result, write) = eval8(cpu, op, dst, src);
            if write {
                cpu.write_reg8(modrm.reg_field, result);
            }
        }
        Width::B16 => {
            let (src, _) = read_rm16(cpu, machine, modrm);
            let dst = cpu.read_reg16(modrm.reg_field);
            let (result, write) = eval16(cpu, op, dst, src);
            if write {
                cpu.write_reg16(modrm.reg_field, result);
            }
        }
        Width::B32 => {
            let (src, _) = read_rm32(cpu, machine, modrm);
            let dst = cpu.read_reg32(modrm.reg_field);
            let (result, write) = eval32(cpu, op, dst, src);
            if write {
                cpu.write_reg32(modrm.reg_field, result);
            }
        }
    }
}

pub fn eval8(cpu: &mut X86Cpu, op: Op, dst: u8, src: u8) -> (u8, bool) {
    match op {
        Op::Add => {
            let (r, cf, of, af) = add8(dst, src);
            apply_arith8(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Adc => {
            let (r, cf, of, af) = adc8(dst, src, carry(cpu));
            apply_arith8(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sub => {
            let (r, cf, of, af) = sub8(dst, src);
            apply_arith8(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sbb => {
            let (r, cf, of, af) = sbb8(dst, src, carry(cpu));
            apply_arith8(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Cmp => {
            let (r, cf, of, af) = sub8(dst, src);
            apply_arith8(cpu, r, cf, of, af);
            (r, false)
        }
        Op::Or => {
            let r = dst | src;
            apply_logic8(cpu, r);
            (r, true)
        }
        Op::And => {
            let r = dst & src;
            apply_logic8(cpu, r);
            (r, true)
        }
        Op::Xor => {
            let r = dst ^ src;
            apply_logic8(cpu, r);
            (r, true)
        }
        Op::Test => {
            let r = dst & src;
            apply_logic8(cpu, r);
            (r, false)
        }
    }
}

pub fn eval16(cpu: &mut X86Cpu, op: Op, dst: u16, src: u16) -> (u16, bool) {
    match op {
        Op::Add => {
            let (r, cf, of, af) = add16(dst, src);
            apply_arith16(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Adc => {
            let (r, cf, of, af) = adc16(dst, src, carry(cpu));
            apply_arith16(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sub => {
            let (r, cf, of, af) = sub16(dst, src);
            apply_arith16(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sbb => {
            let (r, cf, of, af) = sbb16(dst, src, carry(cpu));
            apply_arith16(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Cmp => {
            let (r, cf, of, af) = sub16(dst, src);
            apply_arith16(cpu, r, cf, of, af);
            (r, false)
        }
        Op::Or => {
            let r = dst | src;
            apply_logic16(cpu, r);
            (r, true)
        }
        Op::And => {
            let r = dst & src;
            apply_logic16(cpu, r);
            (r, true)
        }
        Op::Xor => {
            let r = dst ^ src;
            apply_logic16(cpu, r);
            (r, true)
        }
        Op::Test => {
            let r = dst & src;
            apply_logic16(cpu, r);
            (r, false)
        }
    }
}

pub fn eval32(cpu: &mut X86Cpu, op: Op, dst: u32, src: u32) -> (u32, bool) {
    match op {
        Op::Add => {
            let (r, cf, of, af) = add32(dst, src);
            apply_arith32(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Adc => {
            let (r, cf, of, af) = adc32(dst, src, carry(cpu));
            apply_arith32(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sub => {
            let (r, cf, of, af) = sub32(dst, src);
            apply_arith32(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sbb => {
            let (r, cf, of, af) = sbb32(dst, src, carry(cpu));
            apply_arith32(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Cmp => {
            let (r, cf, of, af) = sub32(dst, src);
            apply_arith32(cpu, r, cf, of, af);
            (r, false)
        }
        Op::Or => {
            let r = dst | src;
            apply_logic32(cpu, r);
            (r, true)
        }
        Op::And => {
            let r = dst & src;
            apply_logic32(cpu, r);
            (r, true)
        }
        Op::Xor => {
            let r = dst ^ src;
            apply_logic32(cpu, r);
            (r, true)
        }
        Op::Test => {
            let r = dst & src;
            apply_logic32(cpu, r);
            (r, false)
        }
    }
}
