// Ver: 1
//! Модуль выполнения инструкций процессора
//! Содержит цикл выполнения, обработку префиксов и диспетчеризацию опкодов

use std::error::Error;

use crate::{
    flags,
    instructions::{
        alu, alu32, bcd, control, control32, exchange, extended, extended32, mov, mov32, segment,
        stack, system,
    },
    machine::DosMachine,
    modrm::ModRm,
    video,
};

pub(crate) fn run(machine: &mut DosMachine) -> Result<Option<u8>, Box<dyn Error>> {
    let palette = video::load_vga_palette();
    while !machine.halted {
        let opcode = machine.read_instr_u8(machine.registers.ip());
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
            0xF2 => {
                machine.has_rep_prefix = true; // REPNE
                machine.rep_prefix_type = Some(0xF2)
            }
            0xF3 => {
                machine.has_rep_prefix = true;
                machine.rep_prefix_type = Some(0xF3)
            }

            _ => {
                if machine.has_extended_prefix {
                    execute_0f(machine, opcode);
                } else {
                    execute(machine, opcode);
                }
                machine.has_address_size_prefix = false;
                machine.has_operand_size_prefix = false;
                machine.has_extended_prefix = false;
                machine.has_rep_prefix = false;
                machine.override_segment = None;
                machine.rep_prefix_type = None;
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
        0x01 => {
            if machine.has_operand_size_prefix {
                alu32::add_rm32_r32(machine, &full_bytes);
            } else {
                alu::add_rm16_r16(machine, &full_bytes);
            }
        }
        0x02 => alu::add_r8_rm8(machine, &full_bytes),
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
        0x20 => alu::and_rm8_r8(machine, &full_bytes),
        0x23 => {
            if machine.has_operand_size_prefix {
                alu32::and_rm32_r32(machine, &full_bytes);
            } else {
                alu::and_rm16_r16(machine, &full_bytes);
            }
        }
        0x24 => alu::and_al_imm8(machine, &full_bytes),
        0x2B => {
            if machine.has_operand_size_prefix {
                alu32::sub_r32_rm32(machine, &full_bytes);
            } else {
                alu::sub_r16_rm16(machine, &full_bytes);
            }
        }
        0x2F => bcd::das(machine, &full_bytes),
        0x30 => alu::xor_rm8_r8(machine, &full_bytes),
        0x31 => {
            if machine.has_operand_size_prefix {
                alu32::xor_rm32_r32(machine, &full_bytes);
            } else {
                alu::xor_rm16_r16(machine, &full_bytes);
            }
        }
        0x32 => alu::xor_r8_rm(machine, &full_bytes),
        0x33 => {
            if machine.has_operand_size_prefix {
                alu32::xor_r32_rm32(machine, &full_bytes);
            } else {
                alu::xor_r16_rm16(machine, &full_bytes);
            }
        }
        0x3A => alu::cmp_r8_rm8(machine, &full_bytes),
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
        0x49 => alu::dec_cx(machine, &full_bytes),
        0x4D => alu::dec_bp(machine, &full_bytes),
        0x4E => alu::dec_si(machine, &full_bytes),
        0x50 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_ax(machine);
        }
        0x53 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_bx(machine);
        }
        0x56 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_si(machine);
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
        0x61 => {
            if machine.has_operand_size_prefix {
                stack::popad(machine);
            } else {
                stack::popa(machine);
            }
        }
        0x68 => {
            if machine.has_operand_size_prefix {
                stack::push_imm32(machine, &full_bytes);
            } else {
                stack::push_imm16(machine, &full_bytes);
            }
        }
        0x69 => {
            if machine.has_operand_size_prefix {
                alu32::imul_r32_rm32_imm32(machine, &full_bytes);
            } else {
                alu::imul_r16_rm16_imm16(machine, &full_bytes);
            }
        }
        0x6C => {
            if machine.has_rep_prefix {
                while machine.registers.cx() != 0 {
                    system::insb(machine, &full_bytes);
                    machine
                        .registers
                        .set_cx(machine.registers.cx().wrapping_sub(1));
                }
            } else {
                system::insb(machine, &full_bytes);
            }
        }
        0x6D => {
            if machine.has_rep_prefix {
                while machine.registers.cx() != 0 {
                    system::insw(machine, &full_bytes);
                    machine
                        .registers
                        .set_cx(machine.registers.cx().wrapping_sub(1));
                }
            } else {
                system::insw(machine, &full_bytes);
            }
        }
        0x6E => {
            if machine.has_rep_prefix {
                while machine.registers.cx() != 0 {
                    system::outsb(machine, &full_bytes);
                    machine
                        .registers
                        .set_cx(machine.registers.cx().wrapping_sub(1));
                }
            } else {
                system::outsb(machine, &full_bytes);
            }
        }
        0x6F => {
            if machine.has_rep_prefix {
                if machine.has_operand_size_prefix {
                    while machine.registers.ecx() != 0 {
                        system::outsd(machine, &full_bytes);
                        machine
                            .registers
                            .set_ecx(machine.registers.ecx().wrapping_sub(1));
                    }
                } else {
                    while machine.registers.cx() != 0 {
                        system::outsw(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                if machine.has_operand_size_prefix {
                    system::outsd(machine, &full_bytes);
                } else {
                    system::outsw(machine, &full_bytes);
                }
            }
        }
        0x72 => control::jb(machine, &full_bytes),
        0x73 => control::jae_rel8(machine, &full_bytes),
        0x74 => control::jz(machine, &full_bytes),
        0x75 => control::jne_rel8(machine, &full_bytes),
        0x77 => control::ja(machine, &full_bytes),
        0x79 => control::jns_rel8(machine, &full_bytes),
        0x7C => control::jl_rel8(machine, &full_bytes),
        0x7D => control::jge_rel8(machine, &full_bytes),
        0x7E => control::jle_rel8(machine, &full_bytes),
        0x7F => control::jg_rel8(machine, &full_bytes),
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
                alu32::test_rm32_r32(machine, &full_bytes);
            } else {
                alu::test_rm16_r16(machine, &full_bytes);
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
            if machine.has_operand_size_prefix {
                mov32::mov_rm32_r32(machine, &full_bytes);
            } else {
                mov::mov_rm16_r16(machine, &full_bytes);
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
        0x8D => {
            if machine.has_operand_size_prefix {
                mov32::lea_r32_rm32(machine, &full_bytes);
            } else {
                mov::lea_r16_rm16(machine, &full_bytes);
            }
        }
        0x8E => mov::mov_sreg_rm16(machine, &full_bytes),
        0x8F => {
            stack::pop_rm16(machine, &full_bytes);
        }
        0x90 => system::nop(machine, &full_bytes),

        0x98 => alu::cbw(machine, &full_bytes),
        0x99 => {
            if machine.has_operand_size_prefix {
                alu32::cdq(machine, &full_bytes);
            } else {
                alu::cwd(machine, &full_bytes);
            }
        }
        0x9B => system::wait(machine, &full_bytes),
        0x9A => control::call_far(machine, &full_bytes),
        0x9C => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pushf(machine);
        }
        0x9D => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::popf(machine);
        }
        0x9E => system::sahf(machine, &full_bytes),
        0x9F => system::lahf(machine, &full_bytes),
        0xA0 => {
            if machine.has_address_size_prefix {
                mov::mov_al_address32(machine, &full_bytes);
            } else {
                mov::mov_al_address16(machine, &full_bytes);
            }
        }
        0xA1 => {
            let addr_size = if machine.has_address_size_prefix {
                32
            } else {
                16
            };
            let op_size = if machine.has_operand_size_prefix {
                32
            } else {
                16
            };

            match (addr_size, op_size) {
                (16, 16) => mov::mov_ax_address16(machine, &full_bytes),
                (16, 32) => mov32::mov_eax_address16(machine, &full_bytes),
                (32, 16) => mov::mov_ax_address32(machine, &full_bytes),
                (32, 32) => mov32::mov_eax_address32(machine, &full_bytes),
                _ => {
                    log::error!(
                        "Invalid addr_size/op_size combination: {}/{} at CS:IP={:#04x}:{:#04x}",
                        addr_size,
                        op_size,
                        machine.registers.cs(),
                        machine.registers.ip()
                    );
                    machine.halted = true;
                }
            }
        }
        0xA3 => {
            if machine.has_operand_size_prefix {
                mov32::mov_address_eax(machine, &full_bytes);
            } else {
                mov::mov_address_ax(machine, &full_bytes);
            }
        }
        // libs/dos_core/src/cpu/executor.rs → fn execute()
        0xA4 => {
            if machine.has_rep_prefix {
                // REP MOVSB: повторяем пока CX != 0
                // Оптимизация: блочное копирование для графических операций
                if machine.video.mode == video::VideoMode::Mode13h
                    && machine.registers.es() == 0xA000
                    && machine.registers.ds() == 0xA000
                    && machine.registers.cx() > 1000
                {
                    // Быстрое копирование блока видеопамяти (оптимизация)
                    let si = machine.registers.si() as usize;
                    let di = machine.registers.di() as usize;
                    let cx = machine.registers.cx() as usize;
                    let df = (machine.registers.flags() & (flags::DF)) != 0;

                    let video_size = 320 * 200; // 64000 байт
                    if si.saturating_add(cx) > video_size || di.saturating_add(cx) > video_size {
                        // Выход за пределы видеопамяти — используем стандартную реализацию
                        while machine.registers.cx() != 0 {
                            mov::movsb(machine, &full_bytes);
                            machine
                                .registers
                                .set_cx(machine.registers.cx().wrapping_sub(1));
                        }
                    } else if let Some(fb) = machine.video.framebuffer.as_mut() {
                        if !df && di > si && di < si + cx {
                            // Перекрывающиеся области — копируем назад во избежание порчи данных
                            for i in (0..cx).rev() {
                                fb.data[di + i] = fb.data[si + i];
                            }
                        } else {
                            // Прямое копирование
                            for i in 0..cx {
                                fb.data[di + i] = fb.data[si + i];
                            }
                        }
                        machine.video.dirty = true;

                        // Обновляем регистры
                        if df {
                            machine.registers.set_si((si - cx) as u16);
                            machine.registers.set_di((di - cx) as u16);
                        } else {
                            machine.registers.set_si((si + cx) as u16);
                            machine.registers.set_di((di + cx) as u16);
                        }
                        machine.registers.set_cx(0);
                    } else {
                        // Стандартная реализация без оптимизации
                        while machine.registers.cx() != 0 {
                            mov::movsb(machine, &full_bytes);
                            machine
                                .registers
                                .set_cx(machine.registers.cx().wrapping_sub(1));
                        }
                    }
                } else {
                    // Стандартная реализация REP MOVSB
                    while machine.registers.cx() != 0 {
                        mov::movsb(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                // Однократное выполнение MOVSB
                mov::movsb(machine, &full_bytes);
            }
        }
        0xA6 => {
            if machine.has_rep_prefix {
                // Используем ПРАВИЛЬНОЕ поле для определения типа префикса
                let prefix_type = machine.rep_prefix_type.unwrap_or(0);

                if prefix_type == 0xF3 {
                    // REPE/REPZ — повторять пока совпадают (ZF=1)
                    while machine.registers.cx() != 0 {
                        mov::cmpsb(machine, &full_bytes);
                        let zf = (machine.registers.flags() & (flags::ZF)) != 0;
                        if !zf {
                            break; // остановка при первом несовпадении
                        }
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                } else if prefix_type == 0xF2 {
                    // REPNE/REPNZ — повторять пока не совпадают (ZF=0)
                    while machine.registers.cx() != 0 {
                        mov::cmpsb(machine, &full_bytes);
                        let zf = (machine.registers.flags() & (flags::ZF)) != 0;
                        if zf {
                            break; // остановка при первом совпадении
                        }
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                } else {
                    // Обычный REP (редкий случай) — повторять просто по CX
                    while machine.registers.cx() != 0 {
                        mov::cmpsb(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                // Однократное выполнение CMPSB
                mov::cmpsb(machine, &full_bytes);
            }
        }

        0xA7 => {
            if machine.has_rep_prefix {
                // Определяем тип префикса: 0xF3 = REPE/REPZ, 0xF2 = REPNE/REPNZ
                //let rep_prefix = machine.opcode_override_segment.unwrap_or(0);
                let rep_prefix = machine.rep_prefix_type.unwrap_or(0);
                if rep_prefix == 0xF3 {
                    // REPE/REPZ — повторять пока совпадают (ZF=1)
                    while machine.registers.cx() != 0 {
                        mov::cmpsw(machine, &full_bytes);
                        let zf = (machine.registers.flags() & (flags::ZF)) != 0;
                        if !zf {
                            break; // остановка при первом несовпадении
                        }
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                } else if rep_prefix == 0xF2 {
                    // REPNE/REPNZ — повторять пока не совпадают (ZF=0)
                    while machine.registers.cx() != 0 {
                        mov::cmpsw(machine, &full_bytes);
                        let zf = (machine.registers.flags() & (flags::ZF)) != 0;
                        if zf {
                            break; // остановка при первом совпадении
                        }
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                } else {
                    // Обычный REP (редкий случай) — повторять просто по CX
                    while machine.registers.cx() != 0 {
                        mov::cmpsw(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                // Однократное выполнение CMPSW
                mov::cmpsw(machine, &full_bytes);
            }
        }
        0xA8 => alu::test_al_imm8(machine, &full_bytes),
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
        0xBD => {
            if machine.has_operand_size_prefix {
                mov32::mov_ebp_imm32(machine, &full_bytes); // 32-битная версия
            } else {
                mov::mov_bp_imm16(machine, &full_bytes); // 16-битная версия ← ОСНОВНАЯ
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
        0xC4 => segment::les_r16_m16(machine, &full_bytes),
        0xC6 => mov::mov_rm8_imm8(machine, &full_bytes),
        0xC7 => {
            if machine.has_operand_size_prefix {
                mov32::mov_rm32_imm32(machine, &full_bytes);
            } else {
                mov::mov_rm16_imm16(machine, &full_bytes);
            }
        }
        0xCB => control::retf(machine, &full_bytes),
        0xD1 => {
            if machine.has_operand_size_prefix {
                alu32::shift_group_d1_32(machine, &full_bytes);
            } else {
                alu::shift_group_d1(machine, &full_bytes);
            }
        }
        0xD2 => alu::shift_rm8_cl(machine, &full_bytes),
        0xD3 => {
            if machine.has_operand_size_prefix {
                alu32::shift_rm32_cl(machine, &full_bytes); // 32-битная версия
            } else {
                alu::shift_rm16_cl(machine, &full_bytes); // 16-битная версия ← ОСНОВНАЯ
            }
        }
        0xE0 => control::loopnz_rel8(machine, &full_bytes),
        0xE1 => control::loopz_rel8(machine, &full_bytes),
        0xE2 => control::loop_cx(machine, &full_bytes),
        0xE3 => {
            if machine.has_operand_size_prefix {
                control32::jecxz_rel8(machine, &full_bytes); // 32-битная версия (проверка ECX)
            } else {
                control::jcxz_rel8(machine, &full_bytes); // 16-битная версия (проверка CX)
            }
        }
        0xE6 => system::out_imm8_al(machine, &full_bytes),
        0xE8 => {
            if machine.has_operand_size_prefix {
                control32::call32(machine, &full_bytes);
            } else {
                control::call(machine, &full_bytes);
            }
        }
        0xE9 => control::jmp_rel16(machine, &full_bytes),
        0xEA => control::jmp_far(machine, &full_bytes),
        0xEB => control::jmp_rel8(machine, &full_bytes),
        0xC0 => alu::shift_group_c0_rm8(machine, &full_bytes),
        0xCD => system::int(machine, &full_bytes),
        0xCF => system::iret(machine, &full_bytes),
        0xE4 => system::in_al_imm8(machine, &full_bytes),
        0xEC => system::in_al_dx(machine, &full_bytes),
        0xF4 => system::hlt(machine, &full_bytes),
        0xF5 => system::cmc(machine, &full_bytes),
        0xF6 => alu::group_f6_rm8(machine, &full_bytes),
        0xF7 => {
            if machine.has_operand_size_prefix {
                alu32::group_f7_rm32(machine, &full_bytes); // 32-битная версия (если реализована)
            } else {
                alu::group_f7_rm16(machine, &full_bytes); // 16-битная версия ← НОВОЕ
            }
        }
        0xF8 => {
            system::clc(machine, &full_bytes);
        }
        0xF9 => {
            system::stc(machine, &full_bytes);
        }
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
        0xFD => {
            machine.log_instruction(csip, &full_bytes).ok();
            system::std(machine, &full_bytes);
        }
        0xFE => alu::group_fe_rm8(machine, &full_bytes),
        0xFF => {
            let modrm_byte = machine.read_instr_u8(machine.registers.ip());
            let modrm = ModRm::from_byte(modrm_byte);

            match modrm.reg_field {
                0 => {
                    // INC r/m16/32
                    if machine.has_operand_size_prefix {
                        alu32::inc_rm32(machine, &full_bytes);
                    } else {
                        alu::inc_rm16(machine, &full_bytes);
                    }
                }
                1 => {
                    // DEC r/m16/32
                    if machine.has_operand_size_prefix {
                        alu32::dec_rm32(machine, &full_bytes);
                    } else {
                        alu::dec_rm16(machine, &full_bytes);
                    }
                }
                2 => {
                    // CALL r/m16/32 — вызов через регистр/память (внутрисегментный)
                    if machine.has_operand_size_prefix {
                        control32::call_rm32(machine, &full_bytes);
                    } else {
                        control::call_rm16(machine, &full_bytes);
                    }
                }
                3 => {
                    // CALL ptr16:16 / CALL ptr32:32 — МЕЖСЕГМЕНТНЫЙ ВЫЗОВ через память
                    if machine.has_operand_size_prefix {
                        control32::call_far_rm32(machine, &full_bytes);
                    } else {
                        control::call_far_rm16(machine, &full_bytes);
                    }
                }
                4 => {
                    // JMP r/m16/r/m32 — переход через регистр/память (внутрисегментный)
                    if machine.has_operand_size_prefix {
                        control32::jmp_rm32(machine, &full_bytes);
                    } else {
                        control::jmp_rm16(machine, &full_bytes);
                    }
                }
                5 => {
                    // JMP ptr16:16 / JMP ptr32:32 — МЕЖСЕГМЕНТНЫЙ ПЕРЕХОД через память ← ВАШ СЛУЧАЙ!
                    if machine.has_operand_size_prefix {
                        control32::jmp_far_rm32(machine, &full_bytes);
                    } else {
                        control::jmp_far_rm16(machine, &full_bytes);
                    }
                }
                6 => {
                    // PUSH r/m16/32
                    if machine.has_operand_size_prefix {
                        stack::push_rm32(machine, &full_bytes);
                    } else {
                        stack::push_rm16(machine, &full_bytes);
                    }
                }
                _ => {
                    log::error!(
                        "Unsupported opcode 0xFF with reg_field={} at CS:IP = {:#04x}:{:#04x}",
                        modrm.reg_field,
                        machine.registers.cs(),
                        machine.registers.ip()
                    );
                    machine.halted = true;
                }
            }
        }
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
            if machine.has_operand_size_prefix {
                control32::jz_rel32(machine, &full_bytes); // 32-бит (rel32)
            } else {
                control::jz_rel16(machine, &full_bytes); // 16-бит (rel16) ← НОВАЯ ФУНКЦИЯ
            }
        }
        0xB7 => {
            if machine.has_operand_size_prefix {
                extended32::movzx_r32_rm16(machine, &full_bytes);
            } else {
                extended::movzx_r16_rm8(machine, &full_bytes);
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

#[cfg(test)]
mod tests {
    // tests/executor_comprehensive.rs
    // tests/executor_full.rs
    // Полноценные тесты для всех опкодов из executor.rs
    // Запуск: cargo test --test executor_full -- --nocapture

    #![cfg(test)]
    use super::*;
    use crate::DosMachine;
    use crate::cpu::flags;
    use crate::memory::Memory;
    use std::fs::File;

    // ============================================================================
    // 🛠️ Вспомогательные функции
    // ============================================================================

    /// Создаёт тестовую машину с загруженным кодом по адресу 0x0100
    fn create_test_machine(code: &[u8]) -> DosMachine {
        let mut mem = Memory::new();

        // PSP заглушка: INT 20h по адресу 0
        mem.write_u8(0x00, 0xCD);
        mem.write_u8(0x01, 0x20);

        // Загрузка кода по физическому адресу 0x0100 (CS=0, IP=0x100)
        let base = 0x0100usize;
        for (i, &b) in code.iter().enumerate() {
            mem.write_u8((base + i) as u32, b);
        }

        // Лог-файл (кроссплатформенный null)
        #[cfg(windows)]
        let log = File::create("NUL").unwrap();
        #[cfg(not(windows))]
        let log = File::create("/dev/null").unwrap();

        let mut m = DosMachine::new_with_memory(mem, log);
        m.registers.set_cs(0x0000);
        m.registers.set_ip(0x0100);
        m.registers.set_ds(0x0000);
        m.registers.set_es(0x0000);
        m.registers.set_ss(0x0000);
        m.registers.set_sp(0xFFFE);
        m
    }

    /// Запускает эмуляцию и возвращает финальное состояние
    fn run_code(code: &[u8]) -> DosMachine {
        let mut m = create_test_machine(code);
        let _ = crate::cpu::executor::run(&mut m);
        m
    }

    // ============================================================================
    // 🧪 Макрос для тестов (исправленная версия — без проблем с типами)
    // ============================================================================

    macro_rules! test_opcode {
        ($name:ident, $m:ident, $code:expr, $check:block) => {
            #[test]
            fn $name() {
                // $code может быть vec![...] или блоком { let mut c = vec![...]; c }
                let code_vec: Vec<u8> = $code;
                let machine = run_code(&code_vec);
                let $m: &DosMachine = &machine;
                $check
            }
        };
    }

    // ============================================================================
    // 🔢 1. ALU: ADD, SUB, ADC, SBB, CMP, TEST, AND, OR, XOR
    // ============================================================================

    test_opcode!(
        test_add_reg16_reg16,
        m,
        vec![
            0xB8, 0x11, 0x11, // MOV AX, 1111h
            0xBB, 0x22, 0x22, // MOV BX, 2222h
            0x01, 0xD8, // ADD AX, BX  → AX=3333h
            0xCD, 0x20 // INT 20h
        ],
        {
            assert_eq!(m.registers.ax(), 0x3333);
            assert!(!flags::test_cf(m.registers.flags()));
            assert!(!flags::test_zf(m.registers.flags()));
            assert!(!flags::test_sf(m.registers.flags()));
            assert!(!flags::test_of(m.registers.flags()));
        }
    );

    test_opcode!(
        test_add_mem_imm8,
        m,
        {
            vec![
                0xC6, 0x06, 0x50, 0x00, 0x10, // MOV BYTE [0050h], 10h
                0x80, 0x06, 0x50, 0x00, 0x0A, // ADD BYTE [0050h], 0Ah
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.memory.read_u8(0x0050), 0x1A);
        }
    );

    test_opcode!(
        test_adc_with_carry,
        m,
        vec![
            0xB8, 0xFF, 0xFF, // MOV AX, FFFFh
            0xBB, 0x00, 0x01, // MOV BX, 0001h
            0xF9, // STC (CF=1)
            0x11, 0xD8, // ADC AX, BX → AX=0001h, CF=1, ZF=0
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0001);
            assert!(flags::test_cf(m.registers.flags()));
            assert!(!flags::test_zf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_sub_reg16_set_flags,
        m,
        vec![
            0xB8, 0x10, 0x00, // MOV AX, 0010h
            0xBB, 0x20, 0x00, // MOV BX, 0020h
            0x29, 0xD8, // SUB AX, BX → AX=FFF0h, CF=1, SF=1
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xFFF0);
            assert!(flags::test_cf(m.registers.flags()));
            assert!(flags::test_sf(m.registers.flags()));
            assert!(!flags::test_zf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_cmp_set_zf,
        m,
        vec![
            0xB8, 0x55, 0x55, // MOV AX, 5555h
            0x3D, 0x55, 0x55, // CMP AX, 5555h
            0xCD, 0x20
        ],
        {
            assert!(flags::test_zf(m.registers.flags()));
            assert!(!flags::test_cf(m.registers.flags()));
            assert_eq!(m.registers.ax(), 0x5555); // CMP не меняет операнд
        }
    );

    test_opcode!(
        test_and_reg_mem,
        m,
        {
            vec![
                0xC6, 0x06, 0x60, 0x00, 0xFF, // MOV BYTE [0060h], FFh
                0xB0, 0x0F, // MOV AL, 0Fh
                0x20, 0x06, 0x60, 0x00, // AND [0060h], AL → [60]=0Fh
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.memory.read_u8(0x0060), 0x0F);
            assert!(!flags::test_cf(m.registers.flags()));
            assert!(!flags::test_of(m.registers.flags()));
        }
    );

    test_opcode!(
        test_or_al_imm8,
        m,
        vec![
            0xB0, 0x04, // MOV AL, 04h
            0x0C, 0x02, // OR AL, 02h → AL=06h
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.al(), 0x06);
            assert!(!flags::test_cf(m.registers.flags()));
            assert!(!flags::test_zf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_xor_r8_rm8,
        m,
        vec![
            0xB0, 0xAA, // MOV AL, AAh
            0xB3, 0x55, // MOV BL, 55h
            0x30, 0xD8, // XOR AL, BL → AL=FFh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.al(), 0xFF);
            assert!(flags::test_sf(m.registers.flags()));
            assert!(!flags::test_zf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_test_al_imm8,
        m,
        vec![
            0xB0, 0x05, // MOV AL, 05h (00000101)
            0xA8, 0x04, // TEST AL, 04h → ZF=0 (бит 2 установлен)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.al(), 0x05); // TEST не меняет операнд
            assert!(!flags::test_zf(m.registers.flags()));
            assert!(!flags::test_cf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_neg_ax,
        m,
        vec![
            0xB8, 0x01, 0x00, // MOV AX, 0001h
            0xF7, 0xD8, // NEG AX → AX=FFFFh, CF=1
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xFFFF);
            assert!(flags::test_cf(m.registers.flags()));
        }
    );

    // ============================================================================
    // ➕➖ 2. INC/DEC (флаги, сохранение CF)
    // ============================================================================

    test_opcode!(
        test_inc_preserves_cf,
        m,
        vec![
            0xF9, // STC (CF=1)
            0xB8, 0xFE, 0x7F, // MOV AX, 7FFEh
            0x40, // INC AX → AX=7FFFh, CF=1 (не меняется!)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x7FFF);
            assert!(flags::test_cf(m.registers.flags())); // CF сохранён
            assert!(!flags::test_of(m.registers.flags()));
        }
    );

    test_opcode!(
        test_dec_overflow_flag,
        m,
        vec![
            0xB8, 0x00, 0x80, // MOV AX, 8000h (-32768)
            0x48, // DEC AX → AX=7FFFh (+32767), OF=1
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x7FFF);
            assert!(flags::test_of(m.registers.flags())); // Знаковое переполнение
        }
    );

    // ============================================================================
    // 📦 3. MOV: reg↔reg, reg↔mem, imm, absolute
    // ============================================================================

    test_opcode!(
        test_mov_reg16_reg16,
        m,
        vec![
            0xB8, 0xAB, 0xCD, // MOV AX, CDABh
            0x89, 0xC3, // MOV BX, AX → BX=CDABh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.bx(), 0xCDAB);
        }
    );

    test_opcode!(
        test_mov_rm16_imm16_mem,
        m,
        {
            vec![
                0xC7, 0x06, 0x70, 0x00, 0x12, 0x34, // MOV WORD [0070h], 3412h
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.memory.read_u16(0x0070), 0x3412);
        }
    );

    test_opcode!(
        test_mov_ax_abs16,
        m,
        {
            vec![
                0xC7, 0x06, 0x80, 0x00, 0x99, 0xAA, // MOV WORD [0080h], AA99h
                0xA1, 0x80, 0x00, // MOV AX, [0080h]
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.registers.ax(), 0xAA99);
        }
    );

    test_opcode!(
        test_mov_sreg_rm16,
        m,
        vec![
            0xB8, 0x00, 0x40, // MOV AX, 4000h
            0x8E, 0xD8, // MOV DS, AX
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ds(), 0x4000);
        }
    );

    test_opcode!(
        test_mov_rm16_sreg,
        m,
        vec![
            0x8C, 0xC0, // MOV AX, ES (ES=0 по умолчанию)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0000);
        }
    );

    test_opcode!(
        test_lea_r16_rm16,
        m,
        vec![
            0xBB, 0x10, 0x00, // MOV BX, 0010h
            0x8D, 0x47, 0x05, // LEA AX, [BX+5] → AX=0015h
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0015);
        }
    );

    // ============================================================================
    // 📚 4. STACK: PUSH/POP, PUSHA/POPA, PUSHAD/POPAD
    // ============================================================================

    test_opcode!(
        test_push_pop_ax,
        m,
        vec![
            0xB8, 0xDE, 0xAD, // MOV AX, DEADh
            0x50, // PUSH AX
            0xB8, 0x00, 0x00, // MOV AX, 0000h
            0x58, // POP AX
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xDEAD);
        }
    );

    test_opcode!(
        test_pusha_popa,
        m,
        vec![
            0xB8, 0x01, 0x00, 0xB9, 0x02, 0x00, 0xBA, 0x03, 0x00, 0xBB, 0x04, 0x00, 0xBD, 0x05,
            0x00, 0xBE, 0x06, 0x00, 0xBF, 0x07, 0x00, 0x60, // PUSHA
            0x31, 0xC0, 0x31, 0xC9, 0x31, 0xD2, 0x31, 0xDB, 0x31, 0xED, 0x31, 0xF6, 0x31,
            0xFF, // zero all
            0x61, // POPA
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 1);
            assert_eq!(m.registers.cx(), 2);
            assert_eq!(m.registers.dx(), 3);
            assert_eq!(m.registers.bx(), 4);
            assert_eq!(m.registers.bp(), 5);
            assert_eq!(m.registers.si(), 6);
            assert_eq!(m.registers.di(), 7);
        }
    );

    test_opcode!(
        test_pushad_popad_32,
        m,
        vec![
            0x66, 0xB8, 0x11, 0x22, 0x33, 0x44, // MOV EAX, 44332211h
            0x66, 0xB9, 0x55, 0x66, 0x77, 0x88, // MOV ECX, 88776655h
            0x66, 0x60, // PUSHAD
            0x66, 0x31, 0xC0, 0x66, 0x31, 0xC9, // zero EAX, ECX
            0x66, 0x61, // POPAD
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.eax(), 0x44332211);
            assert_eq!(m.registers.ecx(), 0x88776655);
        }
    );

    test_opcode!(
        test_pushf_popf,
        m,
        vec![
            0xF9, // STC (CF=1)
            0x9C, // PUSHF
            0xFC, // CLD (DF=0)
            0x9D, // POPF
            0xCD, 0x20
        ],
        {
            // После POPF флаги должны восстановиться (включая установленный ранее CF)
            assert!(flags::test_cf(m.registers.flags()));
        }
    );

    // ============================================================================
    // 🔄 5. CONTROL FLOW: JMP, Jcc, CALL, RET, LOOP
    // ============================================================================

    test_opcode!(
        test_jz_taken,
        m,
        vec![
            0xB8, 0x00, 0x00, // MOV AX, 0
            0x3D, 0x00, 0x00, // CMP AX, 0
            0x74, 0x02, // JZ +2 (пропускаем следующий MOV)
            0xB8, 0x00, 0x00, // (пропущен)
            0xB8, 0xDE, 0xAD, // MOV AX, DEADh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xDEAD);
        }
    );

    test_opcode!(
        test_jnz_not_taken,
        m,
        vec![
            0xB8, 0x00, 0x00, // MOV AX, 0
            0x3D, 0x00, 0x00, // CMP AX, 0 → ZF=1
            0x75, 0x02, // JNZ +2 (НЕ выполняется, т.к. ZF=1)
            0xB8, 0xCA, 0xFE, // MOV AX, FECAh (выполнится)
            0xB8, 0xDE, 0xAD, // (пропущен)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xFECA);
        }
    );

    test_opcode!(
        test_loop_cx,
        m,
        vec![
            0xB9, 0x03, 0x00, // MOV CX, 3
            0xB8, 0x00, 0x00, // MOV AX, 0
            0x40, // INC AX
            0xE2, 0xFC, // LOOP -4 (обратно на INC AX)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 3); // CX: 3→2→1→0, AX инкрементирован 3 раза
            assert_eq!(m.registers.cx(), 0);
        }
    );

    test_opcode!(
        test_call_ret,
        m,
        vec![
            0xB8, 0x11, 0x11, // MOV AX, 1111h
            0xE8, 0x03, 0x00, // CALL +3
            0xB8, 0x22, 0x22, // (пропуск)
            0xC3, // RET
            0xB8, 0x33, 0x33, // MOV AX, 3333h (точка возврата)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x3333);
        }
    );

    test_opcode!(
        test_jcxz,
        m,
        vec![
            0xB9, 0x00, 0x00, // MOV CX, 0
            0xE3, 0x02, // JCXZ +2
            0xB8, 0x00, 0x00, // (пропущен)
            0xB8, 0xAB, 0xCD, // MOV AX, CDABh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xCDAB);
        }
    );

    // ============================================================================
    // 🔀 6. PREFIXES: 0x66 (operand), 0x67 (address), REP
    // ============================================================================

    test_opcode!(
        test_66_operand_prefix,
        m,
        vec![
            0x66, 0xB8, 0x12, 0x34, 0x56, 0x78, // MOV EAX, 78563412h
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.eax(), 0x78563412);
            assert_eq!(m.registers.ax(), 0x3412); // младшие 16 бит
        }
    );

    test_opcode!(
        test_movzx_r16_rm8,
        m,
        vec![
            0xB3, 0xFF, // MOV BL, FFh
            0x0F, 0xB6, 0xC3, // MOVZX AX, BL → AX=00FFh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x00FF);
        }
    );

    test_opcode!(
        test_rep_movsb_optimized,
        m,
        {
            let mut code = vec![
                0xFC, // CLD
                0xBE, 0x00, 0x01, // MOV SI, 0100h
                0xBF, 0x10, 0x01, // MOV DI, 0110h
                0xB9, 0x04, 0x00, // MOV CX, 4
            ];
            // Заполняем источник: 01, 02, 03, 04
            code.extend_from_slice(&[0xC6, 0x06, 0x00, 0x01, 0x01]);
            code.extend_from_slice(&[0xC6, 0x06, 0x01, 0x01, 0x02]);
            code.extend_from_slice(&[0xC6, 0x06, 0x02, 0x01, 0x03]);
            code.extend_from_slice(&[0xC6, 0x06, 0x03, 0x01, 0x04]);
            code.extend_from_slice(&[0x26, 0xF3, 0xA4]); // ES:REP MOVSB
            code.extend_from_slice(&[0xCD, 0x20]);
            code
        },
        {
            assert_eq!(m.memory.read_u8(0x0110), 0x01);
            assert_eq!(m.memory.read_u8(0x0111), 0x02);
            assert_eq!(m.memory.read_u8(0x0112), 0x03);
            assert_eq!(m.memory.read_u8(0x0113), 0x04);
            assert_eq!(m.registers.cx(), 0);
            assert_eq!(m.registers.si(), 0x0104);
            assert_eq!(m.registers.di(), 0x0114);
        }
    );

    test_opcode!(
        test_rep_stosb_fill,
        m,
        vec![
            0xFC, // CLD
            0xBF, 0x00, 0x01, // MOV DI, 0100h
            0xB9, 0x03, 0x00, // MOV CX, 3
            0xB0, 0xAA, // MOV AL, AAh
            0xF3, 0xAA, // REP STOSB
            0xCD, 0x20
        ],
        {
            assert_eq!(m.memory.read_u8(0x0100), 0xAA);
            assert_eq!(m.memory.read_u8(0x0101), 0xAA);
            assert_eq!(m.memory.read_u8(0x0102), 0xAA);
            assert_eq!(m.registers.di(), 0x0103);
        }
    );

    // ============================================================================
    // 🧭 7. FLAG MANIPULATION & SYSTEM
    // ============================================================================

    test_opcode!(
        test_clc_stc_cmc,
        m,
        vec![
            0xF8, 0xF9, 0xF5, // CLC, STC, CMC → CF=0
            0xCD, 0x20
        ],
        {
            assert!(!flags::test_cf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_sahf_lahf,
        m,
        vec![
            0xB4, 0x45, // MOV AH, 45h (CF=1, PF=0, AF=1, ZF=0, SF=0)
            0x9E, // SAHF → флаги обновляются
            0x9F, // LAHF → AH = флаги
            0xCD, 0x20
        ],
        {
            let ah = m.registers.ah();
            assert_eq!(ah & 0x01, 0x01); // CF
            assert_eq!(ah & 0x10, 0x10); // AF
        }
    );

    test_opcode!(
        test_cbw,
        m,
        vec![
            0xB0, 0xFF, // MOV AL, FFh (-1)
            0x98, // CBW → AX = FFFFh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xFFFF);
        }
    );

    test_opcode!(
        test_cwd,
        m,
        vec![
            0xB8, 0x00, 0x80, // MOV AX, 8000h (-32768)
            0x99, // CWD → DX = FFFFh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.dx(), 0xFFFF);
        }
    );

    // ============================================================================
    // 🔁 8. SHIFT/ROTATE: SHL, SHR, SAR, ROL, ROR, RCL, RCR
    // ============================================================================

    test_opcode!(
        test_shl_rm16_imm8,
        m,
        vec![
            0xB8, 0x01, 0x00, // MOV AX, 0001h
            0xC1, 0xE0, 0x03, // SHL AX, 3 → AX=0008h, CF=0
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0008);
            assert!(!flags::test_cf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_sar_preserves_sign,
        m,
        vec![
            0xB8, 0x00, 0xFF, // MOV AX, FF00h (-256)
            0xC1, 0xF8, 0x04, // SAR AX, 4 → AX=FFF0h (-16)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xFFF0);
            assert!(flags::test_sf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_rol_16_cl,m,
        vec![
            0xB8, 0x01, 0x80, // MOV AX, 8001h
            0xB1, 0x01, // MOV CL, 1
            0xD1, 0xC0, // ROL AX, 1 → AX=0003h, CF=1, OF=1
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0003);
            assert!(flags::test_cf(m.registers.flags()));
            assert!(flags::test_of(m.registers.flags()));
        }
    );

    // ============================================================================
    // 🧩 9. GROUP OPCODES: 0x80, 0x83, 0xFE, 0xF6, 0xF7
    // ============================================================================

    test_opcode!(
        test_group_x80_add_mem_imm8,
        m,
        {
            vec![
                0xC6, 0x06, 0x90, 0x00, 0x05, // MOV BYTE [0090h], 05h
                0x80, 0x06, 0x90, 0x00, 0x03, // ADD BYTE [0090h], 03h
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.memory.read_u8(0x0090), 0x08);
        }
    );

    test_opcode!(
        test_group_x83_sign_extend,
        m,
        vec![
            0xB8, 0x00, 0x00, // MOV AX, 0
            0x83, 0xC0, 0xFF, // ADD AX, -1 (imm8 sign-extended)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xFFFF);
        }
    );

    test_opcode!(
        test_group_fe_inc_rm8,m,
        {
            vec![
                0xC6, 0x06, 0xA0, 0x00, 0x7F, // MOV BYTE [00A0h], 7Fh
                0xFE, 0x06, 0xA0, 0x00, // INC BYTE [00A0h] → 80h, OF=1
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.memory.read_u8(0x00A0), 0x80);
            assert!(flags::test_of(m.registers.flags()));
        }
    );

    test_opcode!(
        test_group_f6_mul,m,
        vec![
            0xB0, 0x05, // MOV AL, 5
            0xB3, 0x04, // MOV BL, 4
            0xF6, 0xE3, // MUL BL → AX=14h (20), CF=OF=0
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0014);
            assert!(!flags::test_cf(m.registers.flags()));
            assert!(!flags::test_of(m.registers.flags()));
        }
    );

    test_opcode!(
        test_group_f7_div,m,
        vec![
            0xB8, 0x00, 0x0A, // MOV AX, 000Ah
            0xBB, 0x02, 0x00, // MOV BX, 2
            0xF7, 0xF3, // DIV BX → AX=5, DX=0
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 5);
            assert_eq!(m.registers.dx(), 0);
        }
    );

    // ============================================================================
    // 🎯 10. STRING OPS: CMPSB, LODSB, OUTSB/INSB
    // ============================================================================

    test_opcode!(
        test_cmpsb_set_flags,m,
        {
            vec![
                0xC6, 0x06, 0xB0, 0x00, 0x55, // MOV BYTE [00B0h], 55h
                0xC6, 0x06, 0xB1, 0x00, 0x55, // MOV BYTE [00B1h], 55h
                0xBE, 0xB0, 0x00, // MOV SI, 00B0h
                0xBF, 0xB1, 0x00, // MOV DI, 00B1h
                0xA6, // CMPSB → ZF=1
                0xCD, 0x20,
            ]
        },
        {
            assert!(flags::test_zf(m.registers.flags()));
            assert_eq!(m.registers.si(), 0x00B1);
            assert_eq!(m.registers.di(), 0x00B2);
        }
    );

    test_opcode!(
        test_lodsb,m,
        {
            vec![
                0xC6, 0x06, 0xC0, 0x00, 0xAB, // MOV BYTE [00C0h], ABh
                0xBE, 0xC0, 0x00, // MOV SI, 00C0h
                0xAC, // LODSB → AL=ABh, SI=00C1h
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.registers.al(), 0xAB);
            assert_eq!(m.registers.si(), 0x00C1);
        }
    );

    // ============================================================================
    // 🔌 11. I/O: IN, OUT (заглушки)
    // ============================================================================

    test_opcode!(
        test_in_al_imm8_stub,m,
        vec![
            0xE4, 0x60, // IN AL, 60h (клавиатура)
            0xCD, 0x20
        ],
        {
            // Заглушка возвращает сканкод, проверяем только что не упало
            assert!(m.registers.al() != 0xFF);
        }
    );

    test_opcode!(
        test_out_imm8_al_stub,m,
        vec![
            0xB0, 0x55, // MOV AL, 55h
            0xE6, 0x61, // OUT 61h, AL (динамик)
            0xCD, 0x20
        ],
        {
            // Заглушка — проверяем только завершение
            assert!(!m.halted);
        }
    );

    // ============================================================================
    // 🧪 Тесты на префиксы и edge-cases
    // ============================================================================

    test_opcode!(
        test_67_address_prefix_mov,m,
        {
            vec![
                0x67, 0xA1, 0x00, 0x02, 0x00, 0x00, // MOV EAX, [00000200h] (32-bit addr)
                0xCD, 0x20,
            ]
        },
        {
            // В реальном режиме 32-битный адрес усекается до 16 бит
            // Проверяем, что не упало с паникой
            assert!(!m.halted);
        }
    );

    test_opcode!(
        test_segment_override_es,m,
        {
            vec![
                0x26, 0x8A, 0x06, 0xD0, 0x00, // MOV AL, ES:[00D0h]
                0xCD, 0x20,
            ]
        },
        {
            // Проверяем, что override_segment обработан
            assert!(!m.halted);
        }
    );

    test_opcode!(
        test_hlt,m,
        vec![
            0xF4, // HLT
        ],
        {
            assert!(m.halted);
        }
    );

    test_opcode!(
        test_nop,m,
        vec![
            0x90, // NOP
            0xB8, 0xAA, 0xBB, // MOV AX, BBAAh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xBBAA);
        }
    );

    // ============================================================================
    // 🏁 Тест завершения программы
    // ============================================================================

    test_opcode!(
        test_int20_exit,m,
        vec![
            0xCD, 0x20 // INT 20h → halted=true
        ],
        {
            assert!(m.halted);
        }
    );
    test_opcode!(
        test_adc_no_carry,m,
        vec![
            0xF8, // CLC
            0xB8, 0x10, 0x00, // MOV AX, 0010h
            0xBB, 0x20, 0x00, // MOV BX, 0020h
            0x11, 0xD8, // ADC AX, BX → AX=0030h, CF=0
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0030);
            assert!(!flags::test_cf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_sbb_with_borrow,m,
        vec![
            0xF9, // STC (CF=1)
            0xB8, 0x10, 0x00, // MOV AX, 0010h
            0xBB, 0x05, 0x00, // MOV BX, 0005h
            0x19, 0xD8, // SBB AX, BX → AX=0010 - 0005 - 1 = 000Ah
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x000A);
            assert!(!flags::test_cf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_neg_zero,m,
        vec![
            0xB8, 0x00, 0x00, // MOV AX, 0000h
            0xF7, 0xD8, // NEG AX → AX=0000h, CF=0 (special case)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0000);
            assert!(!flags::test_cf(m.registers.flags())); // NEG 0 → CF=0
            assert!(flags::test_zf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_cmp_al_imm8_set_sf,m,
        vec![
            0xB0, 0x05, // MOV AL, 05h
            0x3C, 0x10, // CMP AL, 10h → 05-10 = F5h, SF=1
            0xCD, 0x20
        ],
        {
            assert!(flags::test_sf(m.registers.flags()));
            assert!(flags::test_cf(m.registers.flags())); // 5 < 16 → CF=1
        }
    );

    // ============================================================================
    // 📦 13. MORE MOV: segment loads, LEA edge cases
    // ============================================================================

    test_opcode!(
        test_mov_ds_imm16_via_ax,m,
        vec![
            0xB8, 0x40, 0x00, // MOV AX, 0040h
            0x8E, 0xD8, // MOV DS, AX
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ds(), 0x0040);
        }
    );

    test_opcode!(
        test_lea_bp_disp8,m,
        vec![
            0xBD, 0x00, 0x01, // MOV BP, 0100h
            0x8D, 0x45, 0x10, // LEA AX, [BP+10h] → AX=0110h
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x0110);
        }
    );

    test_opcode!(
        test_mov_rm16_imm16_memory,m,
        {
            vec![
                0xC7, 0x06, 0xE0, 0x00, 0x78, 0x56, // MOV WORD [00E0h], 5678h
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.memory.read_u16(0x00E0), 0x5678);
        }
    );

    // ============================================================================
    // 🔄 14. MORE CONTROL: JA, JBE, JG, JLE with flag combinations
    // ============================================================================

    test_opcode!(
        test_ja_taken,m,
        vec![
            0xB8, 0x20, 0x00, // MOV AX, 0020h
            0x3D, 0x10, 0x00, // CMP AX, 0010h → CF=0, ZF=0
            0x77, 0x02, // JA +2 (jump if above: CF=0 AND ZF=0)
            0xB8, 0x00, 0x00, // (skipped)
            0xB8, 0xAA, 0xBB, // MOV AX, BBAAh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xBBAA);
        }
    );

    test_opcode!(
        test_jle_taken,m,
        vec![
            0xB8, 0x10, 0x00, // MOV AX, 0010h
            0x3D, 0x20, 0x00, // CMP AX, 0020h → SF≠OF or ZF=1
            0x7E, 0x02, // JLE +2 (jump if less or equal)
            0xB8, 0x00, 0x00, // (skipped)
            0xB8, 0xCC, 0xDD, // MOV AX, DDCC h
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xDDCC);
        }
    );

    test_opcode!(
        test_jg_not_taken,m,
        vec![
            0xB8, 0x10, 0x00, // MOV AX, 0010h
            0x3D, 0x20, 0x00, // CMP AX, 0020h → not greater
            0x7F, 0x02, // JG +2 (NOT taken)
            0xB8, 0x11, 0x11, // MOV AX, 1111h (executed)
            0xB8, 0x22, 0x22, // (skipped)
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0x1111);
        }
    );

    // ============================================================================
    // 🔀 15. MORE PREFIXES: REPNE CMPSB, segment overrides
    // ============================================================================

    test_opcode!(
        test_repne_cmpsb_stop_on_match,m,
        {
            let mut code = vec![
                0xFC, // CLD
                0xBE, 0x00, 0x01, // MOV SI, 0100h
                0xBF, 0x10, 0x01, // MOV DI, 0110h
                0xB9, 0x04, 0x00, // MOV CX, 4
            ];
            // Источник: 01, 02, 03, 04
            code.extend_from_slice(&[0xC6, 0x06, 0x00, 0x01, 0x01]);
            code.extend_from_slice(&[0xC6, 0x06, 0x01, 0x01, 0x02]);
            code.extend_from_slice(&[0xC6, 0x06, 0x02, 0x01, 0x03]);
            code.extend_from_slice(&[0xC6, 0x06, 0x03, 0x01, 0x04]);
            // Приёмник: 05, 02, 07, 08 (совпадение на втором байте)
            code.extend_from_slice(&[0xC6, 0x06, 0x10, 0x01, 0x05]);
            code.extend_from_slice(&[0xC6, 0x06, 0x11, 0x01, 0x02]);
            code.extend_from_slice(&[0xC6, 0x06, 0x12, 0x01, 0x07]);
            code.extend_from_slice(&[0xC6, 0x06, 0x13, 0x01, 0x08]);
            // F2 A6 = REPNE CMPSB
            code.extend_from_slice(&[0xF2, 0xA6]);
            code.extend_from_slice(&[0xCD, 0x20]);
            code
        },
        {
            // Остановка при совпадении (второй байт): CX должен быть 2 (4-1-1)
            assert_eq!(m.registers.cx(), 2);
            assert!(flags::test_zf(m.registers.flags())); // ZF=1 при совпадении
            assert_eq!(m.registers.si(), 0x0102);
            assert_eq!(m.registers.di(), 0x0112);
        }
    );

    test_opcode!(
        test_cs_override_mov,m,
        {
            vec![
                0x2E, 0x8A, 0x06, 0x00, 0x01, // MOV AL, CS:[0100h]
                0xCD, 0x20,
            ]
        },
        {
            // Проверяем, что не упало с паникой
            assert!(!m.halted);
        }
    );

    // ============================================================================
    // 🧩 16. GROUP 0xFF: INC/DEC/PUSH memory, CALL/JMP far via memory
    // ============================================================================

    test_opcode!(
        test_ff0_inc_rm16_memory,m,
        {
            vec![
                0xC7, 0x06, 0xF0, 0x00, 0xFF, 0x7F, // MOV WORD [00F0h], 7FFFh
                0xFF, 0x06, 0xF0, 0x00, // INC WORD [00F0h] → 8000h, OF=1
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.memory.read_u16(0x00F0), 0x8000);
            assert!(flags::test_of(m.registers.flags()));
            assert!(flags::test_sf(m.registers.flags()));
        }
    );

    test_opcode!(
        test_ff6_push_rm16_memory,m,
        {
            vec![
                0xC7, 0x06, 0x00, 0x02, 0xAB, 0xCD, // MOV WORD [0200h], CDABh
                0xFF, 0x36, 0x00, 0x02, // PUSH WORD [0200h]
                0x58, // POP AX
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.registers.ax(), 0xCDAB);
        }
    );

    // ============================================================================
    // 🎯 17. STRING: LODSW, STOSW with DF=1
    // ============================================================================

    test_opcode!(
        test_lodsw_df1,m,
        {
            vec![
                0xC7, 0x06, 0x10, 0x01, 0x34, 0x12, // MOV WORD [0110h], 1234h
                0xFD, // STD (DF=1)
                0xBE, 0x12, 0x01, // MOV SI, 0112h
                0xAD, // LODSW → AX=1234h, SI=0110h
                0xCD, 0x20,
            ]
        },
        {
            assert_eq!(m.registers.ax(), 0x1234);
            assert_eq!(m.registers.si(), 0x0110); // SI decreased by 2
        }
    );

    test_opcode!(
        test_stosw_df1_fill,m,
        vec![
            0xFD, // STD (DF=1)
            0xBF, 0x03, 0x01, // MOV DI, 0103h
            0xB8, 0xAA, 0xBB, // MOV AX, BBAAh
            0xAB, // STOSW → [ES:0103]=BB, [ES:0102]=AA, DI=0101h
            0xCD, 0x20
        ],
        {
            assert_eq!(m.memory.read_u8(0x0102), 0xAA);
            assert_eq!(m.memory.read_u8(0x0103), 0xBB);
            assert_eq!(m.registers.di(), 0x0101);
        }
    );

    // ============================================================================
    // 🧪 18. EDGE CASES: zero count shifts, REP with CX=0
    // ============================================================================

    test_opcode!(
        test_shift_cl_zero_no_change,m,
        vec![
            0xB8, 0x55, 0xAA, // MOV AX, AA55h
            0xB1, 0x00, // MOV CL, 0
            0xD3, 0xE0, // SHL AX, CL → no change, flags unchanged
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xAA55);
            // Флаги не должны измениться при count=0 (проверяем, что не упало)
            assert!(!m.halted);
        }
    );

    test_opcode!(
        test_rep_with_cx_zero,m,
        vec![
            0xFC, // CLD
            0xB9, 0x00, 0x00, // MOV CX, 0
            0xF3, 0xA4, // REP MOVSB → не выполняется ни разу
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.cx(), 0);
            assert!(!m.halted);
        }
    );

    // ============================================================================
    // 🏁 19. FINAL: INT 20h exit, HLT, NOP chain
    // ============================================================================

    test_opcode!(
        test_int20_halt,m,
        vec![
            0xCD, 0x20 // INT 20h → halted=true
        ],
        {
            assert!(m.halted);
        }
    );

    test_opcode!(
        test_nop_chain,m,
        vec![
            0x90, 0x90, 0x90, // Three NOPs
            0xB8, 0xDE, 0xAD, // MOV AX, DEADh
            0xCD, 0x20
        ],
        {
            assert_eq!(m.registers.ax(), 0xDEAD);
            assert_eq!(m.registers.ip(), 0x0106); // 3×NOP(1) + MOV(3) + INT(2) = 8 bytes from 0x100
        }
    );
}
