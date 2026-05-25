// Ver: 1 File: ./libs/dos_core/src/cpu/executor.rs
//! Модуль выполнения инструкций процессора
//! Содержит цикл выполнения, обработку префиксов и диспетчеризацию опкодов

use crate::{
    dispatch_op32, flags,
    instructions::{
        alu, alu32, bcd, control, control32, exchange, extended, extended32, incs, mov, mov32,
        segment, stack, system,
    },
    machine::DosMachine,
    modrm::ModRm,
    push_reg16, video,
};

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
pub(crate) fn execute(machine: &mut DosMachine, opcode: u8) {
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

    if let Some(reptype) = machine.rep_prefix_type {
        full_bytes.push(reptype);
    }

    full_bytes.push(opcode);
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - full_bytes.len() as u16,
    ];
    match opcode {
        0x00 => alu::add_rm8_r8(machine, &full_bytes),
        0x01 => dispatch_op32!(
            machine,
            alu32::add_rm32_r32(machine, &full_bytes),
            alu::add_rm16_r16(machine, &full_bytes)
        ),
        0x02 => alu::add_r8_rm8(machine, &full_bytes),
        0x03 => dispatch_op32!(
            machine,
            alu32::add_r32_rm32(machine, &full_bytes),
            alu::add_r16_rm16(machine, &full_bytes)
        ),
        0x04 => alu::add_al_imm8(machine, &full_bytes),
        0x05 => dispatch_op32!(
            machine,
            alu32::add_eax_imm32(machine, &full_bytes),
            alu::add_ax_imm16(machine, &full_bytes)
        ),
        0x06 => {
            stack::push_es(machine);
            machine.log_instruction(csip, &full_bytes).ok();
        }
        0x07 => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pop_es(machine);
        }
        0x08 => alu::or_rm8_r8(machine, &full_bytes),
        0x09 => dispatch_op32!(
            machine,
            alu32::or_rm32_r32(machine, &full_bytes),
            alu::or_rm16_r16(machine, &full_bytes)
        ),
        0x0A => alu::or_r8_rm8(machine, &full_bytes),
        0x0B => dispatch_op32!(
            machine,
            alu32::or_r32_rm32(machine, &full_bytes),
            alu::or_r16_rm16(machine, &full_bytes)
        ),
        0x0C => alu::or_al_imm8(machine, &full_bytes),
        0x0D => dispatch_op32!(
            machine,
            alu32::or_eax_imm32(machine, &full_bytes),
            alu::or_ax_imm16(machine, &full_bytes)
        ),
        0x0E => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_cs(machine);
        }
        0x10 => alu::adc_rm8_r8(machine, &full_bytes),
        0x11 => dispatch_op32!(
            machine,
            alu32::adc_rm32_r32(machine, &full_bytes),
            alu::adc_rm16_r16(machine, &full_bytes)
        ),
        0x13 => dispatch_op32!(
            machine,
            alu32::adc_r32_rm32(machine, &full_bytes),
            alu::adc_r16_rm16(machine, &full_bytes)
        ),
        0x18 => alu::sbb_rm8_r8(machine, &full_bytes),
        0x19 => dispatch_op32!(
            machine,
            alu32::sbb_rm32_r32(machine, &full_bytes),
            alu::sbb_rm16_r16(machine, &full_bytes)
        ),
        0x1E => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::push_ds(machine);
        }
        0x1F => {
            machine.log_instruction(csip, &full_bytes).ok();
            stack::pop_ds(machine);
        }
        0x20 => alu::and_rm8_r8(machine, &full_bytes),
        0x21 => dispatch_op32!(
            machine,
            alu32::and_rm32_r32(machine, &full_bytes),
            alu::and_rm16_r16(machine, &full_bytes)
        ),
        0x23 => dispatch_op32!(
            machine,
            alu32::and_r32_rm32(machine, &full_bytes),
            alu::and_r16_rm16(machine, &full_bytes)
        ),
        0x24 => alu::and_al_imm8(machine, &full_bytes),
        0x2A => alu::sub_r8_rm8(machine, &full_bytes), // 8-битный вариант
        0x2B => dispatch_op32!(
            machine,
            alu32::sub_r32_rm32(machine, &full_bytes),
            alu::sub_r16_rm16(machine, &full_bytes)
        ),
        0x2C => alu::sub_al_imm8(machine, &full_bytes),
        0x2F => bcd::das(machine, &full_bytes),
        0x30 => alu::xor_rm8_r8(machine, &full_bytes),
        0x31 => dispatch_op32!(
            machine,
            alu32::xor_rm32_r32(machine, &full_bytes),
            alu::xor_rm16_r16(machine, &full_bytes)
        ),
        0x32 => alu::xor_r8_rm(machine, &full_bytes),
        0x33 => dispatch_op32!(
            machine,
            alu32::xor_r32_rm32(machine, &full_bytes),
            alu::xor_r16_rm16(machine, &full_bytes)
        ),
        0x38 => alu::cmp_rm8_r8(machine, &full_bytes),
        0x3A => alu::cmp_r8_rm8(machine, &full_bytes),
        0x3B => dispatch_op32!(
            machine,
            alu32::cmp_r32_rm32(machine, &full_bytes),
            alu::cmp_r16_rm16(machine, &full_bytes)
        ),
        0x3C => alu::cmp_al_imm8(machine, &full_bytes),
        0x3D => dispatch_op32!(
            machine,
            alu32::cmp_eax_imm32(machine, &full_bytes),
            alu::cmp_ax_imm16(machine, &full_bytes)
        ),
        0x40 => incs::inc_ax(machine, &full_bytes),
        0x41 => incs::inc_cx(machine, &full_bytes),
        0x42 => incs::inc_dx(machine, &full_bytes),
        0x43 => incs::inc_bx(machine, &full_bytes),
        0x44 => incs::inc_sp(machine, &full_bytes),
        0x45 => incs::inc_bp(machine, &full_bytes),
        0x46 => incs::inc_si(machine, &full_bytes),
        0x47 => incs::inc_di(machine, &full_bytes),
        0x48 => incs::dec_ax(machine, &full_bytes),
        0x49 => incs::dec_cx(machine, &full_bytes),
        0x4A => incs::dec_dx(machine, &full_bytes),
        0x4B => incs::dec_bx(machine, &full_bytes),
        0x4C => incs::dec_sp(machine, &full_bytes),
        0x4D => incs::dec_bp(machine, &full_bytes),
        0x4E => incs::dec_si(machine, &full_bytes),
        0x4F => incs::dec_di(machine, &full_bytes),
        0x50 => stack::push_ax(machine, &full_bytes),
        0x51 => stack::push_cx(machine, &full_bytes),
        0x52 => stack::push_dx(machine, &full_bytes),
        0x53 => stack::push_bx(machine, &full_bytes),
        0x54 => stack::push_sp(machine, &full_bytes),
        0x55 => stack::push_bp(machine, &full_bytes),
        0x56 => stack::push_si(machine, &full_bytes),
        0x57 => stack::push_di(machine, &full_bytes),
        0x58 => stack::pop_ax(machine, &full_bytes),
        0x59 => stack::pop_cx(machine, &full_bytes),
        0x5A => stack::pop_dx(machine, &full_bytes),
        0x5B => stack::pop_bx(machine, &full_bytes),
        0x5C => stack::pop_sp(machine, &full_bytes),
        0x5D => stack::pop_bp(machine, &full_bytes),
        0x5E => stack::pop_si(machine, &full_bytes),
        0x5F => stack::pop_di(machine, &full_bytes),
        0x60 => dispatch_op32!(machine, stack::pushad(machine), stack::pusha(machine)),
        0x61 => dispatch_op32!(machine, stack::popad(machine), stack::popa(machine)),
        0x62 => dispatch_op32!(
            machine,
            control32::bound_r32_rm32(machine, &full_bytes),
            control::bound_r16_rm16(machine, &full_bytes)
        ),
        0x68 => dispatch_op32!(
            machine,
            stack::push_imm32(machine, &full_bytes),
            stack::push_imm16(machine, &full_bytes)
        ),
        0x69 => dispatch_op32!(
            machine,
            alu32::imul_r32_rm32_imm32(machine, &full_bytes),
            alu::imul_r16_rm16_imm16(machine, &full_bytes)
        ),
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
                dispatch_op32!(
                    machine,
                    system::outsd(machine, &full_bytes),
                    system::outsw(machine, &full_bytes)
                );
            }
        }
        0x70 => control::jo_rel8(machine, &full_bytes),
        0x72 => control::jb(machine, &full_bytes),
        0x73 => control::jae_rel8(machine, &full_bytes),
        0x74 => control::jz(machine, &full_bytes),
        0x75 => control::jne_rel8(machine, &full_bytes),
        0x76 => control::jbe_rel8(machine, &full_bytes),
        0x77 => control::ja(machine, &full_bytes),
        0x79 => control::jns_rel8(machine, &full_bytes),
        0x7C => control::jl_rel8(machine, &full_bytes),
        0x7D => control::jge_rel8(machine, &full_bytes),
        0x7E => control::jle_rel8(machine, &full_bytes),
        0x7F => control::jg_rel8(machine, &full_bytes),
        0x80 => alu::group_x80(machine, &full_bytes),
        0x81 => dispatch_op32!(
            machine,
            alu32::group_x81_rm32(machine, &full_bytes),
            alu::group_x81_rm16(machine, &full_bytes)
        ),
        0x83 => dispatch_op32!(
            machine,
            alu32::group_x83_rm32(machine, &full_bytes),
            alu::group_x83_rm16(machine, &full_bytes)
        ),
        0x84 => alu::test_rm8_r8(machine, &full_bytes),
        0x85 => dispatch_op32!(
            machine,
            alu32::test_rm32_r32(machine, &full_bytes),
            alu::test_rm16_r16(machine, &full_bytes)
        ),
        0x87 => dispatch_op32!(
            machine,
            exchange::xchg_rm32_r32(machine, &full_bytes),
            exchange::xchg_rm16_r16(machine, &full_bytes)
        ),
        0x88 => mov::mov_rm8_r8(machine, &full_bytes),
        0x89 => dispatch_op32!(
            machine,
            mov32::mov_rm32_r32(machine, &full_bytes),
            mov::mov_rm16_r16(machine, &full_bytes)
        ),
        0x8A => mov::mov_r8_rm8(machine, &full_bytes),
        0x8B => dispatch_op32!(
            machine,
            mov32::mov_r32_rm32(machine, &full_bytes),
            mov::mov_r16_rm16(machine, &full_bytes)
        ),
        0x8C => mov::mov_rm16_sreg(machine, &full_bytes),
        0x8D => dispatch_op32!(
            machine,
            mov32::lea_r32_rm32(machine, &full_bytes),
            mov::lea_r16_rm16(machine, &full_bytes)
        ),
        0x8E => mov::mov_sreg_rm16(machine, &full_bytes),
        0x8F => {
            stack::pop_rm16(machine, &full_bytes);
        }
        0x90 => system::nop(machine, &full_bytes),
        0x91 => dispatch_op32!(
            machine,
            exchange::xchg_eax_ecx(machine, &full_bytes),
            exchange::xchg_ax_cx(machine, &full_bytes)
        ),
        0x92 => dispatch_op32!(
            machine,
            exchange::xchg_eax_edx(machine, &full_bytes),
            exchange::xchg_ax_dx(machine, &full_bytes)
        ),
        0x93 => dispatch_op32!(
            machine,
            exchange::xchg_eax_ebx(machine, &full_bytes),
            exchange::xchg_ax_bx(machine, &full_bytes)
        ),
        0x94 => dispatch_op32!(
            machine,
            exchange::xchg_eax_esp(machine, &full_bytes),
            exchange::xchg_ax_sp(machine, &full_bytes)
        ),
        0x95 => dispatch_op32!(
            machine,
            exchange::xchg_eax_ebp(machine, &full_bytes),
            exchange::xchg_ax_bp(machine, &full_bytes)
        ),
        0x96 => dispatch_op32!(
            machine,
            exchange::xchg_eax_esi(machine, &full_bytes),
            exchange::xchg_ax_si(machine, &full_bytes)
        ),
        0x97 => dispatch_op32!(
            machine,
            exchange::xchg_eax_edi(machine, &full_bytes),
            exchange::xchg_ax_di(machine, &full_bytes)
        ),

        0x98 => alu::cbw(machine, &full_bytes),
        0x99 => dispatch_op32!(
            machine,
            alu32::cdq(machine, &full_bytes),
            alu::cwd(machine, &full_bytes)
        ),
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
        0xA3 => dispatch_op32!(
            machine,
            mov32::mov_address_eax(machine, &full_bytes),
            mov::mov_address_ax(machine, &full_bytes)
        ),
        0xA4 => {
            if machine.has_rep_prefix {
                if machine.video.mode == video::VideoMode::Mode13h
                    && machine.registers.es() == 0xA000
                    && machine.registers.ds() == 0xA000
                    && machine.registers.cx() > 1000
                {
                    let si = machine.registers.si() as usize;
                    let di = machine.registers.di() as usize;
                    let cx = machine.registers.cx() as usize;
                    let df = (machine.registers.flags() & (flags::DF)) != 0;
                    let video_size = 320 * 200;

                    let out_of_bounds = if df {
                        si < cx - 1 || di < cx - 1
                    } else {
                        si.saturating_add(cx) > video_size || di.saturating_add(cx) > video_size
                    };

                    if out_of_bounds {
                        // Fallback если выходим за границы
                        while machine.registers.cx() != 0 {
                            mov::movsb(machine, &full_bytes);
                            machine
                                .registers
                                .set_cx(machine.registers.cx().wrapping_sub(1));
                        }
                    } else if let Some(fb) = machine.video.framebuffer.as_mut() {
                        if df {
                            // [FIX] Для DF=1 используем цикл, чтобы избежать багов с перекрытием
                            while machine.registers.cx() != 0 {
                                mov::movsb(machine, &full_bytes);
                                machine
                                    .registers
                                    .set_cx(machine.registers.cx().wrapping_sub(1));
                            }
                        } else {
                            // DF=0: Оптимизация для прямого направления
                            if di > si && di < si + cx {
                                // Перекрывающиеся области (приемник внутри источника) — копируем назад
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
                            machine.registers.set_si((si + cx) as u16);
                            machine.registers.set_di((di + cx) as u16);
                            machine.registers.set_cx(0);
                        }
                    } else {
                        // Нет доступа к framebuffer — стандартный цикл
                        while machine.registers.cx() != 0 {
                            mov::movsb(machine, &full_bytes);
                            machine
                                .registers
                                .set_cx(machine.registers.cx().wrapping_sub(1));
                        }
                    }
                } else {
                    while machine.registers.cx() != 0 {
                        mov::movsb(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                mov::movsb(machine, &full_bytes);
            }
        }
        0xA5 => {
            if machine.has_rep_prefix {
                if machine.video.mode == video::VideoMode::Mode13h
                    && machine.registers.es() == 0xA000
                    && machine.registers.ds() == 0xA000
                    && machine.registers.cx() > 500
                {
                    let si = machine.registers.si() as usize;
                    let di = machine.registers.di() as usize;
                    let cx = machine.registers.cx() as usize;
                    let df = (machine.registers.flags() & (flags::DF)) != 0;
                    let video_size = 320 * 200 / 2; // в словах

                    // Проверка выхода за границы видеопамяти (в словах)
                    let out_of_bounds = if df {
                        si / 2 < cx.saturating_sub(1) || di / 2 < cx.saturating_sub(1)
                    } else {
                        si / 2 + cx > video_size || di / 2 + cx > video_size
                    };

                    if out_of_bounds {
                        while machine.registers.cx() != 0 {
                            mov::movsw(machine, &full_bytes);
                            machine
                                .registers
                                .set_cx(machine.registers.cx().wrapping_sub(1));
                        }
                    } else if let Some(fb) = machine.video.framebuffer.as_mut() {
                        if df {
                            // DF=1: используем обычный цикл для корректного направления
                            while machine.registers.cx() != 0 {
                                mov::movsw(machine, &full_bytes);
                                machine
                                    .registers
                                    .set_cx(machine.registers.cx().wrapping_sub(1));
                            }
                        } else {
                            if di > si && di < si + cx * 2 {
                                for i in (0..cx).rev() {
                                    let src_idx = si / 2 + i;
                                    let dst_idx = di / 2 + i;
                                    let word = u16::from_le_bytes([
                                        fb.data[src_idx * 2],
                                        fb.data[src_idx * 2 + 1],
                                    ]);
                                    fb.data[dst_idx * 2] = word as u8;
                                    fb.data[dst_idx * 2 + 1] = (word >> 8) as u8;
                                }
                            } else {
                                for i in 0..cx {
                                    let src_idx = si / 2 + i;
                                    let dst_idx = di / 2 + i;
                                    let word = u16::from_le_bytes([
                                        fb.data[src_idx * 2],
                                        fb.data[src_idx * 2 + 1],
                                    ]);
                                    fb.data[dst_idx * 2] = word as u8;
                                    fb.data[dst_idx * 2 + 1] = (word >> 8) as u8;
                                }
                            }
                            machine.video.dirty = true;
                            machine.registers.set_si((si + cx * 2) as u16);
                            machine.registers.set_di((di + cx * 2) as u16);
                            machine.registers.set_cx(0);
                        }
                    } else {
                        while machine.registers.cx() != 0 {
                            mov::movsw(machine, &full_bytes);
                            machine
                                .registers
                                .set_cx(machine.registers.cx().wrapping_sub(1));
                        }
                    }
                } else {
                    while machine.registers.cx() != 0 {
                        mov::movsw(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                mov::movsw(machine, &full_bytes);
            }
        }
        0xA6 => {
            if machine.has_rep_prefix {
                let prefix_type = machine.rep_prefix_type.unwrap_or(0);

                if prefix_type == 0xF3 {
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
                    while machine.registers.cx() != 0 {
                        mov::cmpsb(machine, &full_bytes);
                        machine
                            .registers
                            .set_cx(machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                mov::cmpsb(machine, &full_bytes);
            }
        }

        0xA7 => {
            if machine.has_rep_prefix {
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
        0xA9 => dispatch_op32!(
            machine,
            alu32::test_eax_imm32(machine, &full_bytes),
            alu::test_ax_imm16(machine, &full_bytes)
        ),
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
        0xAE => mov::scasb(machine, &full_bytes),
        0xAF => mov::scasw(machine, &full_bytes),
        0xB0 => mov::mov_al_imm8(machine, &full_bytes),
        0xB1 => mov::mov_cl_imm8(machine, &full_bytes),
        0xB2 => mov::mov_dl_imm8(machine, &full_bytes),
        0xB3 => mov::mov_bl_imm8(machine, &full_bytes),
        0xB4 => mov::mov_ah_imm8(machine, &full_bytes),
        0xB5 => mov::mov_ch_imm8(machine, &full_bytes),
        0xB6 => mov::mov_dh_imm8(machine, &full_bytes),
        0xB7 => mov::mov_bh_imm8(machine, &full_bytes),
        0xB8 => dispatch_op32!(
            machine,
            mov32::mov_eax_data(machine, &full_bytes),
            mov::mov_ax(machine, &full_bytes)
        ),
        0xB9 => mov::mov_cx_imm16(machine, &full_bytes),
        0xBA => {
            if !machine.has_operand_size_prefix {
                mov::mov_dx(machine, &full_bytes);
            } else {
                mov32::mov_edx_data(machine, &full_bytes);
            }
        }
        0xBB => dispatch_op32!(
            machine,
            mov32::mov_ebx_data(machine, &full_bytes),
            mov::mov_bx(machine, &full_bytes)
        ),
        0xBC => dispatch_op32!(
            machine,
            mov32::mov_esp_imm32(machine, &full_bytes),
            mov::mov_sp_imm16(machine, &full_bytes)
        ),
        0xBD => {
            if machine.has_operand_size_prefix {
                mov32::mov_ebp_imm32(machine, &full_bytes); // 32-битная версия
            } else {
                mov::mov_bp_imm16(machine, &full_bytes); // 16-битная версия ← ОСНОВНАЯ
            }
        }
        0xBE => dispatch_op32!(
            machine,
            mov32::mov_esi_imm32(machine, &full_bytes),
            mov::mov_si_imm16(machine, &full_bytes)
        ),
        0xBF => dispatch_op32!(
            machine,
            mov32::mov_edi_imm32(machine, &full_bytes),
            mov::mov_di_imm16(machine, &full_bytes)
        ),
        0xC1 => dispatch_op32!(
            machine,
            alu32::shift_group_c1_32(machine, &full_bytes),
            alu::shift_group_c1_16(machine, &full_bytes)
        ),
        0xC2 => dispatch_op32!(
            machine,
            control32::retn_imm32(machine, &full_bytes),
            control::retn_imm16(machine, &full_bytes)
        ),
        0xC3 => dispatch_op32!(
            machine,
            control32::retn32(machine, &full_bytes),
            control::retn(machine, &full_bytes)
        ),
        0xC4 => segment::les_r16_m16(machine, &full_bytes),
        0xC5 => segment::lds_r16_m16(machine, &full_bytes),
        0xC6 => mov::mov_rm8_imm8(machine, &full_bytes),
        0xC7 => dispatch_op32!(
            machine,
            mov32::mov_rm32_imm32(machine, &full_bytes),
            mov::mov_rm16_imm16(machine, &full_bytes)
        ),
        0xCB => control::retf(machine, &full_bytes),
        0xCC => system::int3(machine, &full_bytes),
        0xD0 => alu::shift_group_d0_rm8(machine, &full_bytes),
        0xD1 => dispatch_op32!(
            machine,
            alu32::shift_group_d1_32(machine, &full_bytes),
            alu::shift_group_d1(machine, &full_bytes)
        ),
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
        0xE8 => dispatch_op32!(
            machine,
            control32::call32(machine, &full_bytes),
            control::call(machine, &full_bytes)
        ),
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
            crate::flags::clear_df(&mut machine.registers.flags());
        }
        0xFA => {
            machine.log_instruction(csip, &full_bytes).ok();
            crate::flags::clear_if(&mut machine.registers.flags());
        }
        0xFB => {
            machine.log_instruction(csip, &full_bytes).ok();
            crate::flags::set_if(&mut machine.registers.flags());
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
                0 => dispatch_op32!(
                    machine,
                    incs::inc_rm32(machine, &full_bytes),
                    incs::inc_rm16(machine, &full_bytes)
                ),
                1 => dispatch_op32!(
                    machine,
                    incs::dec_rm32(machine, &full_bytes),
                    incs::dec_rm16(machine, &full_bytes)
                ),
                2 => dispatch_op32!(
                    machine,
                    control32::call_rm32(machine, &full_bytes),
                    control::call_rm16(machine, &full_bytes)
                ),
                3 => dispatch_op32!(
                    machine,
                    control32::call_far_rm32(machine, &full_bytes),
                    control::call_far_rm16(machine, &full_bytes)
                ),
                4 => dispatch_op32!(
                    machine,
                    control32::jmp_rm32(machine, &full_bytes),
                    control::jmp_rm16(machine, &full_bytes)
                ),
                5 => dispatch_op32!(
                    machine,
                    control32::jmp_far_rm32(machine, &full_bytes),
                    control::jmp_far_rm16(machine, &full_bytes)
                ),
                6 => dispatch_op32!(
                    machine,
                    stack::push_rm32(machine, &full_bytes),
                    stack::push_rm16(machine, &full_bytes)
                ),
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
