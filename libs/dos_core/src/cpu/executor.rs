// Ver: 6
//! Модуль выполнения инструкций процессора
//! Содержит цикл выполнения, обработку префиксов и диспетчеризацию опкодов

use std::{error::Error, process::exit};

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
    //let debug = DebugLog::new("debug.log");
    while !machine.halted {
        let opcode = machine.read_instr_u8(machine.registers.ip());
        machine.registers.step(None);

        match opcode {
            0x0F => machine.has_extended_prefix = true,
            0x67 => machine.has_address_size_prefix = true,
            0x66 => machine.has_operand_size_prefix = true,
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
            0xF0 => {
                machine.has_lock_prefix = true; // REPNE
                //machine.rep_prefix_type = Some(0xF0)
            }
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
                machine.has_lock_prefix = false;
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

/*struct DebugLog {
    logfile: File
}

impl DebugLog {
    pub(crate) fn new<T: ToString>(logname: T) -> Self {
        let logfile = File::create_new(logname.to_string()).expect("not create log Debug file");
        Self { logfile }
    }
    fn print(&self, machine: &DosMachine, segment: u16, offset: u16, size: u8) {
        let csip = [machine.registers.cs(), machine.registers.ip()];
        let read_csip = [segment, offset];
        let mut v = Vec::with_capacity(size as usize);
        for i in 0..size {
            v.push(machine.read_u8(segment, offset+i as u16));
        }
        let hex_bytes: Vec<String> = v.iter().map(|b| format!("{:02X}", b)).collect();
        writeln!(
            &self.logfile,
            "{:#04x}:{:#04x}:\t{:#04x}:{:#04x}:\t{}",
            csip[0],
            csip[1],
            read_csip[0],
            read_csip[1],
            hex_bytes.join(" ")
        ).ok();
        let _ = &self.logfile.sync_all().ok();
    }
    fn print_text<T: ToString>(&self, msg: T) {
        writeln!(
            &self.logfile,
            "{}",
            msg.to_string()
        ).ok();
    }
}*/

/// Диспетчеризация базовых опкодов (без префикса 0x0F)
fn execute(machine: &mut DosMachine, opcode: u8) {
    //, debug: Option<&DebugLog>) {
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
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - full_bytes.len() as u16,
    ];
    /*if let Some(dl) = debug {
        dl.print(machine, 0x1000, 0x3bd, 14);
    }*/
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
        0x0A => alu::or_r8_rm8(machine, &full_bytes),
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
        0x10 => alu::adc_rm8_r8(machine, &full_bytes),
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
        0x21 => {
            if machine.has_operand_size_prefix {
                alu32::and_rm32_r32(machine, &full_bytes);
            } else {
                alu::and_rm16_r16(machine, &full_bytes);
            }
        }
        0x23 => {
            if machine.has_operand_size_prefix {
                alu32::and_r32_rm32(machine, &full_bytes);
            } else {
                alu::and_r16_rm16(machine, &full_bytes);
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
        0x38 => alu::cmp_rm8_r8(machine, &full_bytes),
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
        0x40 => alu::inc_ax(machine, &full_bytes),
        0x45 => alu::inc_bp(machine, &full_bytes),
        0x47 => alu::inc_di(machine, &full_bytes),
        0x48 => alu::dec_ax(machine, &full_bytes),
        0x49 => alu::dec_cx(machine, &full_bytes),
        0x4D => alu::dec_bp(machine, &full_bytes),
        0x4E => alu::dec_si(machine, &full_bytes),
        0x50 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_ax(machine);
        }
        0x51 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_cx(machine);
        }
        0x52 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_dx(machine);
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
        0x62 => {
            if machine.has_operand_size_prefix {
                control32::bound_r32_rm32(machine, &full_bytes);
            } else {
                control::bound_r16_rm16(machine, &full_bytes);
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
        0x70 => control::jb(machine, &full_bytes),
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
        0xA2 => mov::mov_address_al(machine, &full_bytes),
        0xA3 => {
            if machine.has_operand_size_prefix {
                mov32::mov_address_eax(machine, &full_bytes);
            } else {
                mov::mov_address_ax(machine, &full_bytes);
            }
        }
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
        0xA9 => {
            if machine.has_operand_size_prefix {
                alu32::test_eax_imm32(machine, &full_bytes);
            } else {
                alu::test_ax_imm16(machine, &full_bytes);
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
    if machine.has_lock_prefix {
        log::debug!(
            "LOCK prefix consumed at CS:IP={:#04x}:{:#04x}",
            csip[0],
            csip[1]
        );
    }
}

/// Диспетчеризация расширенных опкодов (с префиксом 0x0F)
fn execute_0f(machine: &mut DosMachine, opcode: u8 /* ,debug: Option<&DebugLog>*/) {
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
        0x01 => {
            let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
            let modrm = ModRm::from_byte(modrm_byte);

            match modrm.reg_field {
                1 => {
                    if (modrm.mod_field & 0b1100_0000) == 0b0000_0000 && modrm.rm_field == 0b010 {
                        log::warn!("LGDT stub: Ignored (Real Mode)");
                        machine.registers.step(None);
                        if modrm.mod_field == 0b00 && modrm.rm_field == 0b110 {
                            machine.registers.step(Some(2)); // Пропустить disp16
                        }
                    } else {
                        control::smsw(machine, &full_bytes);
                    }
                }
                2 => {
                    log::info!("LIDT/SIDT stub: Ignored (Real Mode)");
                    machine.registers.step(None);
                }
                4 => {
                    control::smsw(machine, &full_bytes);
                }
                _ => {
                    log::warn!(
                        "Unhandled 0F 01 /{} at CS:IP={:#04x}:{:#04x}",
                        modrm.reg_field,
                        machine.registers.cs(),
                        machine.registers.ip()
                    );
                }
            }
        }
        0xA1 => {
            stack::pop_fs(machine);
            machine
                .log_instruction(
                    [
                        machine.registers.cs(),
                        machine.registers.ip() - full_bytes.len() as u16,
                    ],
                    &full_bytes,
                )
                .ok();
        }
        0x20 => extended::mov_reg32_crn(machine, &full_bytes),
        0x22 => extended::mov_crn_reg32(machine, &full_bytes),
        0x82 => {
            if machine.has_operand_size_prefix {
                control32::jb_rel32(machine, &full_bytes);
            } else {
                control::jb_rel16(machine, &full_bytes);
            }
        }
        0x83 => {
            if machine.has_operand_size_prefix {
                control32::jae_rel32(machine, &full_bytes);
            } else {
                control::jae_rel16(machine, &full_bytes);
            }
        }
        0x84 => {
            if machine.has_operand_size_prefix {
                control32::jz_rel32(machine, &full_bytes);
            } else {
                control::jz_rel16(machine, &full_bytes);
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
