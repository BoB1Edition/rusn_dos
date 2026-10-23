use crate::cpu::X86Cpu;
use crate::modrm::ModRm;
use bus::Machine;

use super::core::Op;
use super::eval::{eval8, eval16, eval32};
use super::util::{read_rm8, read_rm16, read_rm32, write_rm8, write_rm16, write_rm32};

pub fn group80(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    group_imm(cpu, machine, false, false);
}

pub fn group81(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    group_imm(cpu, machine, true, false);
}

pub fn group83(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    group_imm(cpu, machine, true, true);
}

fn group_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine, wide: bool, sign8: bool) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let op = match modrm.reg_field {
        0 => Op::Add,
        1 => Op::Or,
        2 => Op::Adc,
        3 => Op::Sbb,
        4 => Op::And,
        5 => Op::Sub,
        6 => Op::Xor,
        _ => Op::Cmp,
    };
    if !wide {
        let (dst, addr) = read_rm8(cpu, machine, modrm);
        let imm = cpu.fetch_u8(machine);
        let (result, write) = eval8(cpu, op, dst, imm);
        if write {
            write_rm8(cpu, machine, modrm, addr, result);
        }
        return;
    }
    let bits32 = cpu.prefixes.has_operand_size;
    if bits32 {
        let (dst, addr) = read_rm32(cpu, machine, modrm);
        let imm = if sign8 {
            cpu.fetch_u8(machine) as i8 as i32 as u32
        } else {
            cpu.fetch_u32(machine)
        };
        let (result, write) = eval32(cpu, op, dst, imm);
        if write {
            write_rm32(cpu, machine, modrm, addr, result);
        }
    } else {
        let (dst, addr) = read_rm16(cpu, machine, modrm);
        let imm = if sign8 {
            cpu.fetch_u8(machine) as i8 as i16 as u16
        } else {
            cpu.fetch_u16(machine)
        };
        let (result, write) = eval16(cpu, op, dst, imm);
        if write {
            write_rm16(cpu, machine, modrm, addr, result);
        }
    }
}
