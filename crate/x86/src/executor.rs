// Ver: 2 File: crate/x86/src/executor.rs
use crate::cpu::X86Cpu;
use crate::instructions::{alu, control, exchange, group, mov, shift, stack, system};
use bus::Machine;

pub fn execute(cpu: &mut X86Cpu, machine: &mut dyn Machine, opcode: u8) {
    let is_32bit_op = cpu.prefixes.has_operand_size;
    match opcode {
        0x00 => alu::add_rm8_r8(cpu, machine),
        0x01 => {
            if is_32bit_op {
                alu::add_rm32_r32(cpu, machine);
            } else {
                alu::add_rm16_r16(cpu, machine);
            }
        }
        0x02 => alu::add_r8_rm8(cpu, machine),
        0x03 => {
            if is_32bit_op {
                alu::add_r32_rm32(cpu, machine);
            } else {
                alu::add_r16_rm16(cpu, machine);
            }
        }
        0x04 => alu::add_al_imm8(cpu, machine),
        0x05 => {
            if is_32bit_op {
                alu::add_eax_imm32(cpu, machine);
            } else {
                alu::add_ax_imm16(cpu, machine);
            }
        }
        0x06 => stack::push_es(cpu, machine),
        0x07 => stack::pop_es(cpu, machine),
        0x08 => alu::or_rm8_r8(cpu, machine),
        0x09 => {
            if is_32bit_op {
                alu::or_rm32_r32(cpu, machine);
            } else {
                alu::or_rm16_r16(cpu, machine);
            }
        }
        0x0A => alu::or_r8_rm8(cpu, machine),
        0x0B => {
            if is_32bit_op {
                alu::or_r32_rm32(cpu, machine);
            } else {
                alu::or_r16_rm16(cpu, machine);
            }
        }
        0x0C => alu::or_al_imm8(cpu, machine),
        0x0D => {
            if is_32bit_op {
                alu::or_eax_imm32(cpu, machine);
            } else {
                alu::or_ax_imm16(cpu, machine);
            }
        }
        0x0E => stack::push_cs(cpu, machine),
        0x10 => alu::adc_rm8_r8(cpu, machine),
        0x11 => {
            if is_32bit_op {
                alu::adc_rm32_r32(cpu, machine);
            } else {
                alu::adc_rm16_r16(cpu, machine);
            }
        }
        0x12 => alu::adc_r8_rm8(cpu, machine),
        0x13 => {
            if is_32bit_op {
                alu::adc_r32_rm32(cpu, machine);
            } else {
                alu::adc_r16_rm16(cpu, machine);
            }
        }
        0x14 => alu::adc_al_imm8(cpu, machine),
        0x15 => {
            if is_32bit_op {
                alu::adc_eax_imm32(cpu, machine);
            } else {
                alu::adc_ax_imm16(cpu, machine);
            }
        }
        0x16 => stack::push_ss(cpu, machine),
        0x17 => stack::pop_ss(cpu, machine),
        0x18 => alu::sbb_rm8_r8(cpu, machine),
        0x19 => {
            if is_32bit_op {
                alu::sbb_rm32_r32(cpu, machine);
            } else {
                alu::sbb_rm16_r16(cpu, machine);
            }
        }
        0x1A => alu::sbb_r8_rm8(cpu, machine),
        0x1B => {
            if is_32bit_op {
                alu::sbb_r32_rm32(cpu, machine);
            } else {
                alu::sbb_r16_rm16(cpu, machine);
            }
        }
        0x1C => alu::sbb_al_imm8(cpu, machine),
        0x1D => {
            if is_32bit_op {
                alu::sbb_eax_imm32(cpu, machine);
            } else {
                alu::sbb_ax_imm16(cpu, machine);
            }
        }
        0x1E => stack::push_ds(cpu, machine),
        0x1F => stack::pop_ds(cpu, machine),
        0x20 => alu::and_rm8_r8(cpu, machine),
        0x21 => {
            if is_32bit_op {
                alu::and_rm32_r32(cpu, machine);
            } else {
                alu::and_rm16_r16(cpu, machine);
            }
        }
        0x22 => alu::and_r8_rm8(cpu, machine),
        0x23 => {
            if is_32bit_op {
                alu::and_r32_rm32(cpu, machine);
            } else {
                alu::and_r16_rm16(cpu, machine);
            }
        }
        0x24 => alu::and_al_imm8(cpu, machine),
        0x25 => {
            if is_32bit_op {
                alu::and_eax_imm32(cpu, machine);
            } else {
                alu::and_ax_imm16(cpu, machine);
            }
        }
        0x27 => alu::daa(cpu, machine),
        0x28 => alu::sub_rm8_r8(cpu, machine),
        0x29 => {
            if is_32bit_op {
                alu::sub_rm32_r32(cpu, machine);
            } else {
                alu::sub_rm16_r16(cpu, machine);
            }
        }
        0x2A => alu::sub_r8_rm8(cpu, machine),
        0x2B => {
            if is_32bit_op {
                alu::sub_r32_rm32(cpu, machine);
            } else {
                alu::sub_r16_rm16(cpu, machine);
            }
        }
        0x2C => alu::sub_al_imm8(cpu, machine),
        0x2D => {
            if is_32bit_op {
                alu::sub_eax_imm32(cpu, machine);
            } else {
                alu::sub_ax_imm16(cpu, machine);
            }
        }
        0x2F => alu::das(cpu, machine),
        0x30 => alu::xor_rm8_r8(cpu, machine),
        0x31 => {
            if is_32bit_op {
                alu::xor_rm32_r32(cpu, machine);
            } else {
                alu::xor_rm16_r16(cpu, machine);
            }
        }
        0x32 => alu::xor_r8_rm8(cpu, machine),
        0x33 => {
            if is_32bit_op {
                alu::xor_r32_rm32(cpu, machine);
            } else {
                alu::xor_r16_rm16(cpu, machine);
            }
        }
        0x34 => alu::xor_al_imm8(cpu, machine),
        0x35 => {
            if is_32bit_op {
                alu::xor_eax_imm32(cpu, machine);
            } else {
                alu::xor_ax_imm16(cpu, machine);
            }
        }
        0x38 => alu::cmp_rm8_r8(cpu, machine),
        0x39 => {
            if is_32bit_op {
                alu::cmp_rm32_r32(cpu, machine);
            } else {
                alu::cmp_rm16_r16(cpu, machine);
            }
        }
        0x3A => alu::cmp_r8_rm8(cpu, machine),
        0x3B => {
            if is_32bit_op {
                alu::cmp_r32_rm32(cpu, machine);
            } else {
                alu::cmp_r16_rm16(cpu, machine);
            }
        }
        0x3C => alu::cmp_al_imm8(cpu, machine),
        0x3D => {
            if is_32bit_op {
                alu::cmp_eax_imm32(cpu, machine);
            } else {
                alu::cmp_ax_imm16(cpu, machine);
            }
        }
        0x40..=0x47 => alu::inc_reg(cpu, opcode - 0x40),
        0x48..=0x4F => alu::dec_reg(cpu, opcode - 0x48),
        0x50..=0x57 => stack::push_reg(cpu, machine, opcode - 0x50),
        0x58..=0x5F => stack::pop_reg(cpu, machine, opcode - 0x58),
        0x60 => stack::pusha(cpu, machine),
        0x61 => stack::popa(cpu, machine),
        0x62 => control::bound(cpu, machine),
        0x63 => system::arpl(cpu, machine),
        0x69 => system::imul_imm(cpu, machine, false),
        0x6B => system::imul_imm(cpu, machine, true),
        0x68 => stack::push_imm(cpu, machine),
        0x6A => stack::push_imm8(cpu, machine),
        0x6C => mov::insb(cpu, machine),
        0x6D => mov::insw(cpu, machine),
        0x6E => mov::outsb(cpu, machine),
        0x6F => mov::outsw(cpu, machine),
        0x84 => alu::test_rm8_r8(cpu, machine),
        0x85 => {
            if is_32bit_op {
                alu::test_rm32_r32(cpu, machine);
            } else {
                alu::test_rm16_r16(cpu, machine);
            }
        }
        0x86 => exchange::xchg_rm8_r8(cpu, machine),
        0x87 => exchange::xchg_rm_r(cpu, machine),
        0x88 => mov::mov_rm8_r8(cpu, machine),
        0x89 => {
            if is_32bit_op {
                mov::mov_rm32_r32(cpu, machine);
            } else {
                mov::mov_rm16_r16(cpu, machine);
            }
        }
        0x8A => mov::mov_r8_rm8(cpu, machine),
        0x8B => {
            if is_32bit_op {
                mov::mov_r32_rm32(cpu, machine);
            } else {
                mov::mov_r16_rm16(cpu, machine);
            }
        }
        0x8C => mov::mov_rm16_sreg(cpu, machine),
        0x8D => mov::lea(cpu, machine),
        0x8E => mov::mov_sreg_rm16(cpu, machine),
        0xB0..=0xB7 => {
            let reg_idx = opcode - 0xB0;
            let imm = cpu.fetch_u8(machine);
            cpu.write_reg8(reg_idx, imm);
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
        0x8F => stack::pop_rm(cpu, machine),
        0x90 => system::nop(cpu, machine),
        0x91..=0x97 => exchange::xchg_ax_reg(cpu, opcode - 0x90),
        0x98 => system::cbw(cpu, machine),
        0x99 => system::cwd(cpu, machine),
        0x9B => system::wait(cpu, machine),
        0x9E => system::sahf(cpu, machine),
        0x9F => system::lahf(cpu, machine),
        0x70..=0x7F => control::jcc_rel8(cpu, machine, opcode),
        0xC2 => control::ret_imm(cpu, machine),
        0xC3 => control::ret_near0(cpu, machine),
        0xC8 => control::enter(cpu, machine),
        0xC9 => control::leave(cpu, machine),
        0xCA => control::retf_imm(cpu, machine),
        0xCB => control::retf(cpu, machine),
        0xE0 => control::loopnz(cpu, machine),
        0xE1 => control::loopz(cpu, machine),
        0xE2 => control::loop_rel(cpu, machine),
        0xE3 => control::jcxz(cpu, machine),
        0xE8 => control::call_rel(cpu, machine),
        0xE9 => control::jmp_rel(cpu, machine),
        0xEA => control::jmp_far(cpu, machine),
        0xEB => control::jmp_rel8(cpu, machine),
        0x9A => control::call_far(cpu, machine),
        0x9C => stack::pushf(cpu, machine),
        0x9D => stack::popf(cpu, machine),
        0xC4 => mov::les(cpu, machine),
        0xC5 => mov::lds(cpu, machine),
        0xC6 => mov::mov_rm8_imm8(cpu, machine),
        0xC7 => mov::mov_rm_imm(cpu, machine),
        0xCC => system::int3(cpu, machine),
        0xCD => system::int(cpu, machine),
        0xCE => system::into(cpu, machine),
        0xCF => system::iret(cpu, machine),
        0xD4 => system::aam(cpu, machine),
        0xD5 => system::aad(cpu, machine),
        0xD6 => system::salc(cpu, machine),
        0xD7 => system::xlat(cpu, machine),
        0xE4 => system::in_al_imm8(cpu, machine),
        0xE5 => system::in_ax_imm8(cpu, machine),
        0xE6 => system::out_imm8_al(cpu, machine),
        0xE7 => system::out_imm8_ax(cpu, machine),
        0xEC => system::in_al_dx(cpu, machine),
        0xED => system::in_ax_dx(cpu, machine),
        0xEE => system::out_dx_al(cpu, machine),
        0xEF => system::out_dx_ax(cpu, machine),
        0xF1 => system::int1(cpu, machine),
        0xF4 => system::hlt(cpu, machine),
        0xF5 => system::cmc(cpu, machine),
        0xF8 => system::clc(cpu, machine),
        0xF9 => system::stc(cpu, machine),
        0xFC => system::cld(cpu, machine),
        0xFD => system::std(cpu, machine),
        0xFA => system::cli(cpu, machine),
        0xFB => system::sti(cpu, machine),

        // MOV
        0xA0 => mov::mov_al_address(cpu, machine),
        0xA1 => mov::mov_ax_moffs(cpu, machine),
        0xA2 => mov::mov_address_al(cpu, machine),
        0xA3 => mov::mov_moffs_ax(cpu, machine),
        0xA4 => mov::movsb(cpu, machine),
        0xA5 => mov::movsw(cpu, machine),
        0xA8 => alu::test_al_imm8(cpu, machine),
        0xA9 => {
            if is_32bit_op {
                alu::test_eax_imm32(cpu, machine);
            } else {
                alu::test_ax_imm16(cpu, machine);
            }
        }
        0xA6 => mov::cmpsb(cpu, machine),
        0xA7 => mov::cmpsw(cpu, machine),
        0xAA => mov::stosb(cpu, machine),
        0xAB => mov::stosw(cpu, machine),
        0xAC => mov::lodsb(cpu, machine),
        0xAD => mov::lodsw(cpu, machine),
        0xAE => mov::scasb(cpu, machine),
        0xAF => mov::scasw(cpu, machine),
        0x80 | 0x82 => alu::group80(cpu, machine),
        0x81 => alu::group81(cpu, machine),
        0x83 => alu::group83(cpu, machine),
        0xC0 => shift::shift8(cpu, machine, shift::Count::Imm8),
        0xC1 => shift::shift_wide(cpu, machine, shift::Count::Imm8),
        0xD0 => shift::shift8(cpu, machine, shift::Count::One),
        0xD1 => shift::shift_wide(cpu, machine, shift::Count::One),
        0xD2 => shift::shift8(cpu, machine, shift::Count::Cl),
        0xD3 => shift::shift_wide(cpu, machine, shift::Count::Cl),
        0xF6 => group::group_f6(cpu, machine),
        0xF7 => group::group_f7(cpu, machine),
        0xFE => group::group_fe(cpu, machine),
        0xFF => group::group_ff(cpu, machine),

        _ => halt_unknown(cpu, opcode),
    }
}

pub fn execute_0f(cpu: &mut X86Cpu, machine: &mut dyn Machine, opcode: u8) {
    match opcode {
        0x01 => system::group_0f01(cpu, machine),
        0x20 => system::mov_r32_cr(cpu, machine),
        0x22 => system::mov_cr_r32(cpu, machine),
        0x82 => control::jcc_rel16(cpu, machine, 0x72),
        0x83 => control::jcc_rel16(cpu, machine, 0x73),
        0x84 => control::jcc_rel16(cpu, machine, 0x74),
        0xA1 => system::pop_fs(cpu, machine),
        0xB7 => system::movzx(cpu, machine),
        _ => {
            log::error!(
                "Unsupported 0F opcode {:#04X} at CS:IP = {:04X}:{:04X}",
                opcode,
                cpu.registers.cs(),
                cpu.registers.ip().wrapping_sub(2)
            );
            cpu.halted = true;
        }
    }
}

fn halt_unknown(cpu: &mut X86Cpu, opcode: u8) {
    log::error!(
        "Unsupported opcode {:#04X} at CS:IP = {:04X}:{:04X}",
        opcode,
        cpu.registers.cs(),
        cpu.registers.ip().wrapping_sub(1)
    );
    cpu.halted = true;
}
