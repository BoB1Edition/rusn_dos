// Ver: 7
//! Модуль выполнения инструкций процессора
//! Содержит цикл выполнения, обработку префиксов и диспетчеризацию опкодов

use std::error::Error;

use crate::{
    instructions::{
        alu, alu32, bcd, control, control32, exchange, extended, extended32, mov, mov32, stack,
        system,
    },
    machine::DosMachine,
    modrm::ModRm,
    video,
};

/// Основной цикл выполнения программы
pub(crate) fn run(machine: &mut DosMachine) -> Result<Option<u8>, Box<dyn Error>> {
    let palette = video::load_vga_palette();
    while !machine.halted {
        let opcode = machine.read_u8(machine.registers.cs(), machine.registers.ip());
        machine.registers.step(None);

        match opcode {
            0x67 => machine.has_address_size_prefix = true,
            0x66 => machine.has_operand_size_prefix = true,
            0x0F => machine.has_extended_prefix = true,
            0x26 => {
                machine.override_segment = Some(machine.registers.es());
                machine.opcode_override_segment = Some(opcode);
            } // ES:
            0x2E => {
                machine.override_segment = Some(machine.registers.cs());
                machine.opcode_override_segment = Some(opcode);
            } // CS:
            0x36 => {
                machine.override_segment = Some(machine.registers.ss());
                machine.opcode_override_segment = Some(opcode);
            } // SS:
            0x3E => {
                machine.override_segment = Some(machine.registers.ds());
                machine.opcode_override_segment = Some(opcode);
            } // DS:
            0x64 => {
                machine.override_segment = Some(machine.registers.fs());
                machine.opcode_override_segment = Some(opcode);
            } // FS:
            0x65 => {
                machine.override_segment = Some(machine.registers.gs());
                machine.opcode_override_segment = Some(opcode);
            } // GS:
            0xF3 => {
                machine.has_rep_prefix = true;
            }

            _ => {
                if machine.has_extended_prefix {
                    execute_0f(machine, opcode);
                } else {
                    execute(machine, opcode);
                }
                // Сброс флагов префиксов после выполнения инструкции
                machine.has_address_size_prefix = false;
                machine.has_operand_size_prefix = false;
                machine.has_extended_prefix = false;
                machine.has_rep_prefix = false;
                machine.override_segment = None;
                machine.opcode_override_segment = None;
            }
        }
        if machine.video.mode == video::VideoMode::Mode13h && machine.video.dirty {
            if let Some(fb) = &machine.video.framebuffer {
                let scaled = video::upscale_framebuffer(&fb.data, &palette);
                if let Some(window) = machine.window() {
                    let (width, height) = window.get_size();
                    (*window).update_with_buffer(&scaled, width, height)?;
                }
                machine.video.dirty = false;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
    Ok(Some(machine.registers.al()))
}

/// Диспетчеризация базовых опкодов (без префикса 0x0F)
fn execute(machine: &mut DosMachine, opcode: u8) {
    let mut full_bytes = Vec::new();
    if machine.has_operand_size_prefix {
        full_bytes.push(0x66);
    }
    if machine.has_address_size_prefix {
        full_bytes.push(0x67);
    }
    if let Some(oos) = machine.opcode_override_segment {
        full_bytes.push(oos);
    }
    full_bytes.push(opcode);
    let csip = [machine.registers.cs(), machine.registers.ip()];

    match opcode {
        0x00 => alu::add_rm8_r8(machine, &full_bytes),
        0x61 => {
            if machine.has_operand_size_prefix {
                stack::popad(machine);
            } else {
                stack::popa(machine);
            }
        }
        0x01 => {
            if machine.has_operand_size_prefix {
                alu32::add_rm32_r32(machine, &full_bytes);
            } else {
                alu::add_rm16_r16(machine, &full_bytes);
            }
        }
        0x03 => {
            if machine.has_operand_size_prefix {
                alu32::add_r32_rm32(machine, &full_bytes);
            } else {
                alu::add_r16_rm16(machine, &full_bytes);
            }
        }
        0x04 => alu::add_al_imm8(machine, &full_bytes),
        0x05 => {
            if machine.has_operand_size_prefix {
                alu32::add_eax_imm32(machine, &full_bytes);
            } else {
                alu::add_ax_imm16(machine, &full_bytes);
            }
        }
        0x06 => {
            let es = machine.registers.es();
            machine
                .registers
                .set_sp(machine.registers.sp().wrapping_sub(2));
            machine.write_u16(machine.registers.ss(), machine.registers.sp(), es);
            machine.log_instruction(csip, &full_bytes).ok();
        }
        0x07 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pop_es(machine);
        }
        0x08 => alu::or_rm8_r8(machine, &full_bytes),
        0x09 => {
            if machine.has_operand_size_prefix {
                alu32::or_rm32_r32(machine, &full_bytes);
            } else {
                alu::or_rm16_r16(machine, &full_bytes);
            }
        }
        0x0B => {
            if machine.has_operand_size_prefix {
                alu32::or_r32_rm32(machine, &full_bytes);
            } else {
                alu::or_r16_rm16(machine, &full_bytes);
            }
        }
        0x0C => alu::or_al_imm8(machine, &full_bytes),
        0x0E => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_cs(machine);
        }
        0x18 => alu::sbb_rm8_r8(machine, &full_bytes),
        0x1E => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_ds(machine);
        }
        0x1F => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pop_ds(machine);
        }
        0x23 => {
            if machine.has_operand_size_prefix {
                alu32::and_rm32_r32(machine, &full_bytes);
            } else {
                alu::and_rm16_r16(machine, &full_bytes);
            }
        }
        0x2B => {
            if machine.has_operand_size_prefix {
                alu32::sub_r32_rm32(machine, &full_bytes);
            } else {
                alu::sub_r16_rm16(machine, &full_bytes);
            }
        }
        0x2F => bcd::das(machine, &full_bytes),
        0x32 => alu::xor_r8_rm(machine, &full_bytes),
        0x31 => {
            if machine.has_operand_size_prefix {
                alu32::xor_rm32_r32(machine, &full_bytes);
            } else {
                alu::xor_rm16_r16(machine, &full_bytes);
            }
        }
        0x33 => {
            if machine.has_operand_size_prefix {
                alu32::xor_r32_rm32(machine, &full_bytes);
            } else {
                alu::xor_r16_rm16(machine, &full_bytes);
            }
        }
        0x3B => {
            if machine.has_operand_size_prefix {
                alu32::cmp_r32_rm32(machine, &full_bytes);
            } else {
                alu::cmp_r16_rm16(machine, &full_bytes);
            }
        }
        0x3C => alu::cmp_al_imm8(machine, &full_bytes),
        0x3D => {
            if machine.has_operand_size_prefix {
                alu32::cmp_eax_imm32(machine, &full_bytes);
            } else {
                alu::cmp_ax_imm16(machine, &full_bytes);
            }
        }
        0x45 => alu::inc_bp(machine, &full_bytes),
        0x48 => alu::dec_ax(machine, &full_bytes),
        0x4E => alu::dec_si(machine, &full_bytes),
        0x50 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_ax(machine);
        }
        0x53 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_bx(machine);
        }
        0x57 => stack::push_di(machine, &full_bytes),
        0x58 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pop_ax(machine);
        }
        0x5F => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pop_di(machine);
        }
        0x60 => {
            if machine.has_operand_size_prefix {
                stack::pushad(machine);
            } else {
                stack::pusha(machine);
            }
        }
        0x72 => control::jb(machine, &full_bytes),
        0x73 => control::jae_rel8(machine, &full_bytes),
        0x74 => control::jz(machine, &full_bytes),
        0x75 => control::jne_rel8(machine, &full_bytes),
        0x77 => control::ja(machine, &full_bytes),
        // libs/dos_core/src/cpu/executor.rs → fn execute()
        0x7D => control::jge_rel8(machine, &full_bytes),
        0x80 => alu::group_x80(machine, &full_bytes),
        0x83 => {
            if machine.has_operand_size_prefix {
                alu32::group_x83_rm32(machine, &full_bytes);
            } else {
                alu::group_x83_rm16(machine, &full_bytes);
            }
        }
        0x84 => alu::test_rm8_r8(machine, &full_bytes),
        0x85 => {
            if machine.has_operand_size_prefix {
                alu32::test_rm32_r32(machine, &full_bytes); // 32-битная версия (если реализована)
            } else {
                alu::test_rm16_r16(machine, &full_bytes); // 16-битная версия ← НОВОЕ
            }
        }
        0x87 => {
            if machine.has_operand_size_prefix {
                exchange::xchg_rm32_r32(machine, &full_bytes);
            } else {
                exchange::xchg_rm16_r16(machine, &full_bytes);
            }
        }
        0x88 => mov::mov_rm8_r8(machine, &full_bytes),
        0x89 => {
            if !machine.has_operand_size_prefix {
                mov::mov_rm16_r16(machine, &full_bytes);
            } else {
                mov32::mov_rm32_r32(machine, &full_bytes);
            }
        }
        0x8A => mov::mov_r8_rm8(machine, &full_bytes),
        0x8B => {
            if machine.has_operand_size_prefix {
                mov32::mov_r32_rm32(machine, &full_bytes);
            } else {
                mov::mov_r16_rm16(machine, &full_bytes);
            }
        }
        0x8C => mov::mov_rm16_sreg(machine, &full_bytes),
        0x8E => mov::mov_sreg_rm16(machine, &full_bytes),
        0x90 => system::nop(machine, &full_bytes),
        0x9C => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pushf(machine);
        }
        0x9D => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::popf(machine);
        }
        0xA0 => {
            if machine.has_address_size_prefix {
                mov::mov_al_address32(machine, &full_bytes);
            } else {
                mov::mov_al_address16(machine, &full_bytes);
            }
        }
        // libs/dos_core/src/cpu/executor.rs → fn execute()
        0xA1 => {
            if machine.has_operand_size_prefix {
                mov32::mov_eax_address32(machine, &full_bytes);
            } else {
                // Для 16-битного режима: читаем слово в AX (стандартное поведение 8086)
                mov::mov_ax_address16(machine, &full_bytes);
            }
        }
        0xA3 => {
            if machine.has_operand_size_prefix {
                mov32::mov_address_eax(machine, &full_bytes);
            } else {
                mov::mov_address_ax(machine, &full_bytes);
            }
        }
        0xAA => {
            if machine.has_rep_prefix {
                // REP STOSB: повторяем пока CX != 0
                // Оптимизация: заливка всего экрана за один проход (если применимо)
                if machine.video.mode == video::VideoMode::Mode13h
                    && machine.registers.es() == 0xA000
                    && machine.registers.di() == 0
                    && machine.registers.cx() == 320 * 200
                {
                    // Быстрая заливка всего экрана (оптимизация для режима 13h)
                    if let Some(fb) = machine.video.framebuffer.as_mut() {
                        let color = machine.registers.al();
                        for i in 0..(320 * 200) {
                            fb.data[i] = color;
                        }
                        machine.video.dirty = true;
                        machine.registers.set_cx(0);
                        machine.registers.set_di(64000); // DI после 64000 байт
                    }
                } else {
                    // Стандартная реализация REP STOSB
                    while machine.registers.cx() != 0 {
                        mov::stosb(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                // Однократное выполнение STOSB
                mov::stosb(machine, &full_bytes);
            }
        }
        0xAB => {
            if machine.has_rep_prefix {
                // REP STOSW: повторяем пока CX != 0
                // Оптимизация: заливка всего экрана за один проход
                if machine.video.mode == video::VideoMode::Mode13h
                    && machine.registers.es() == 0xA000
                    && machine.registers.di() == 0
                    && machine.registers.cx() == 32000
                {
                    // Быстрая заливка всего экрана (оптимизация для режима 13h)
                    if let Some(fb) = machine.video.framebuffer.as_mut() {
                        let color = machine.registers.al(); // младший байт AX = цвет пикселя
                        for i in 0..(320 * 200) {
                            fb.data[i] = color;
                        }
                        machine.video.dirty = true;
                        machine.registers.set_cx(0);
                        machine.registers.set_di(64000); // 32000 слов × 2 байта
                    }
                } else {
                    // Стандартная реализация REP STOSW
                    while machine.registers.cx() != 0 {
                        mov::stosw(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                // Однократное выполнение STOSW
                mov::stosw(machine, &full_bytes);
            }
        }
        0xAC => {
            if machine.has_rep_prefix {
                // REP LODSB: повторяем пока CX != 0
                while machine.registers.cx() != 0 {
                    mov::lodsb(machine, &full_bytes);
                    machine
                        .registers
                        .set_cx(machine.registers.cx().wrapping_sub(1));
                }
            } else {
                // Однократное выполнение LODSB
                mov::lodsb(machine, &full_bytes);
            }
        }
        0xB0 => mov::mov_al_imm8(machine, &full_bytes),
        0xB2 => mov::mov_dl(machine, &full_bytes),
        0xB3 => mov::mov_bl_imm8(machine, &full_bytes),
        0xB4 => mov::mov_ah(machine, &full_bytes),
        0xB7 => mov::mov_bh_imm8(machine, &full_bytes),
        0xB8 => {
            if machine.has_operand_size_prefix {
                mov32::mov_eax_data(machine, &full_bytes);
            } else {
                mov::mov_ax(machine, &full_bytes);
            }
        }
        0xB9 => mov::mov_cx_imm16(machine, &full_bytes),
        0xBA => {
            if !machine.has_operand_size_prefix {
                mov::mov_dx(machine, &full_bytes);
            } else {
                mov32::mov_edx_data(machine, &full_bytes);
            }
        }
        0xBB => {
            if machine.has_operand_size_prefix {
                mov32::mov_ebx_data(machine, &full_bytes);
            } else {
                mov::mov_bx(machine, &full_bytes);
            }
        }
        0xBE => {
            if machine.has_operand_size_prefix {
                mov32::mov_esi_imm32(machine, &full_bytes);
            } else {
                mov::mov_si_imm16(machine, &full_bytes);
            }
        }
        0xBF => {
            if machine.has_operand_size_prefix {
                mov32::mov_edi_imm32(machine, &full_bytes);
            } else {
                mov::mov_di_imm16(machine, &full_bytes);
            }
        }
        0xC1 => {
            if machine.has_operand_size_prefix {
                alu32::shift_group_c1_32(machine, &full_bytes);
            } else {
                alu::shift_group_c1_16(machine, &full_bytes);
            }
        }
        0xC3 => {
            if machine.has_operand_size_prefix {
                control32::retn32(machine, &full_bytes);
            } else {
                control::retn(machine, &full_bytes);
            }
        }
        0xC7 => {
            if machine.has_operand_size_prefix {
                mov32::mov_rm32_imm32(machine, &full_bytes);
            } else {
                mov::mov_rm16_imm16(machine, &full_bytes);
            }
        }
        0xD1 => {
            if machine.has_operand_size_prefix {
                alu32::shift_group_d1_32(machine, &full_bytes);
            } else {
                alu::shift_group_d1(machine, &full_bytes);
            }
        }
        0xE2 => control::loop_cx(machine, &full_bytes),
        0xE8 => {
            if machine.has_operand_size_prefix {
                control32::call32(machine, &full_bytes);
            } else {
                control::call(machine, &full_bytes);
            }
        }
        0xE9 => control::jmp_rel16(machine, &full_bytes),
        0xEB => control::jmp_rel8(machine, &full_bytes),
        0xC0 => alu::shift_group_c0_rm8(machine, &full_bytes),
        0xF7 => {
            if machine.has_operand_size_prefix {
                alu32::group_f7_rm32(machine, &full_bytes); // 32-битная версия (если реализована)
            } else {
                alu::group_f7_rm16(machine, &full_bytes); // 16-битная версия ← НОВОЕ
            }
        }
        0xFF => {
            let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
            let modrm = ModRm::from_byte(modrm_byte);
            match modrm.reg_field {
                2 => {
                    // CALL r/m16/32
                    if machine.has_operand_size_prefix {
                        control32::call_rm32(machine, &full_bytes);
                    } else {
                        control::call_rm16(machine, &full_bytes);
                    }
                }
                4 => {
                    // JMP r/m16/r/m32
                    if machine.has_operand_size_prefix {
                        control32::jmp_rm32(machine, &full_bytes);
                    } else {
                        control::jmp_rm16(machine, &full_bytes);
                    }
                }
                _ => machine.print_error_exit(opcode),
            }
        }
        0xCD => system::int(machine, &full_bytes),
        0xE4 => system::in_al_imm8(machine, &full_bytes),
        0xEC => system::in_al_dx(machine, &full_bytes),
        0xF4 => system::hlt(machine, &full_bytes),
        0xF6 => alu::group_f6_rm8(machine, &full_bytes),
        0xFC => {
            machine.log_instruction(csip, &full_bytes).ok();
            crate::cpu::flags::clear_df(&mut machine.registers.flags());
        }
        0xFA => {
            machine.log_instruction(csip, &full_bytes).ok();
            crate::cpu::flags::clear_if(&mut machine.registers.flags());
        }
        0xFB => {
            machine.log_instruction(csip, &full_bytes).ok();
            crate::cpu::flags::set_if(&mut machine.registers.flags());
        }
        0xFE => alu::group_fe_rm8(machine, &full_bytes),
        _ => machine.print_error_exit(opcode),
    }
}

/// Диспетчеризация расширенных опкодов (с префиксом 0x0F)
fn execute_0f(machine: &mut DosMachine, opcode: u8) {
    let mut full_bytes = Vec::new();
    if machine.has_operand_size_prefix {
        full_bytes.push(0x66);
    }
    if machine.has_address_size_prefix {
        full_bytes.push(0x67);
    }
    if let Some(oos) = machine.opcode_override_segment {
        full_bytes.push(oos);
    }
    full_bytes.push(0x0F);
    full_bytes.push(opcode);

    match opcode {
        0x01 => control::smsw(machine, &full_bytes),
        0xA1 => {
            stack::pop_fs(machine);
            machine
                .log_instruction(
                    [machine.registers.cs(), machine.registers.ip()],
                    &full_bytes,
                )
                .ok();
        }
        0x84 => {
            // JZ/JE rel32 — условный переход при ZF=1
            control32::jz_rel32(machine, &full_bytes);
        }
        0xB7 => {
            if machine.has_operand_size_prefix {
                extended::movzx_r16_rm16(machine, &full_bytes);
            } else {
                extended32::movzx_r32_rm16(machine, &full_bytes);
            }
        }
        _ => {
            log::error!(
                "Unsupported opcode0f {:#02X} at CS:IP = {:#04x}:{:#04x}",
                opcode,
                machine.registers.cs(),
                machine.registers.ip()
            );
            machine.halted = true;
        }
    }
}
