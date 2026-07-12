// Ver: 2 File: crate/x86/src/executor.rs
use crate::cpu::X86Cpu;
use crate::instructions::{mov, system};
use bus::Machine;

pub fn execute(cpu: &mut X86Cpu, machine: &mut dyn Machine, opcode: u8) {
    let is_32bit_op = cpu.prefixes.has_operand_size;
    //let is_32bit_addr = cpu.prefixes.has_address_size;
    match opcode {
        // System
        0x88 => mov::mov_rm8_r8(cpu, machine),
        0x8A => mov::mov_r8_rm8(cpu, machine),

        0x89 => {
            if is_32bit_op {
                //mov32::mov_rm32_r32(cpu, machine);
                //log::warn!("MOV r/m32, r32 (0x89 with 0x66) not fully migrated yet");
            } else {
                mov::mov_rm16_r16(cpu, machine);
            }
        }
        0x8B => {
            if is_32bit_op {
                //mov32::mov_r32_rm32(cpu, machine);
                //log::warn!("MOV r32, r/m32 (0x8B with 0x66) not fully migrated yet");
            } else {
                mov::mov_r16_rm16(cpu, machine);
            }
        }

        // === MOV reg, imm (Пример учета 0x66) ===
        0xB8..=0xBF => {
            let reg_idx = opcode - 0xB8;
            if is_32bit_op {
                let imm = cpu.fetch_u32(machine);
                cpu.write_reg32(reg_idx, imm);
            } else {
                let imm = cpu.fetch_u16(machine);
                cpu.write_reg16(reg_idx, imm);
            }
        }
        0x90 => system::nop(cpu, machine),
        0xF4 => system::hlt(cpu, machine),
        0xCD => system::int(cpu, machine),
        0xCF => system::iret(cpu, machine),
        0xEC => system::in_al_dx(cpu, machine),
        0xE4 => system::in_al_imm8(cpu, machine),
        0xE6 => system::out_imm8_al(cpu, machine),
        0xF8 => system::clc(cpu, machine),
        0xF9 => system::stc(cpu, machine),
        0xFC => system::cld(cpu, machine),
        0xFD => system::std(cpu, machine),
        0xFA => system::cli(cpu, machine),
        0xFB => system::sti(cpu, machine),

        // MOV
        0x88 => mov::mov_rm8_r8(cpu, machine),
        0x8A => mov::mov_r8_rm8(cpu, machine),
        0x89 => mov::mov_rm16_r16(cpu, machine),
        0x8B => mov::mov_r16_rm16(cpu, machine),
        0xA4 => mov::movsb(cpu, machine),
        0xAA => mov::stosb(cpu, machine),

        // MOV reg8, imm8 (0xB0 - 0xB7)
        0xB0..=0xB7 => {
            let reg_idx = opcode - 0xB0;
            let imm = cpu.fetch_u8(machine);
            cpu.write_reg8(reg_idx, imm);
        }

        // MOV reg16, imm16 (0xB8 - 0xBF)
        0xB8..=0xBF => {
            let reg_idx = opcode - 0xB8;
            let imm = cpu.fetch_u16(machine);
            cpu.write_reg16(reg_idx, imm);
        }

        _ => {
            log::error!(
                "Unsupported opcode {:#04X} at CS:IP = {:04X}:{:04X}",
                opcode,
                cpu.registers.cs(),
                cpu.registers.ip().wrapping_sub(1)
            );
            cpu.halted = true;
        }
    }
}
