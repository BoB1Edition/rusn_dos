// Ver: 4 File: ./libs/dos_core/src/instructions/macros/mod.rs

#[macro_export]
macro_rules! inc_reg16 {
    ($name:ident, $get:ident, $set:ident, $opcode:literal) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();

            let old = machine.registers.$get();
            let result = old.wrapping_add(1);
            let af = ((old & 0x0F) + 1) > 0x0F;
            let of = old == 0x7FFF;
            let current_cf = (machine.registers.flags() & $crate::flags::CF) != 0;

            machine.registers.$set(result);
            machine
                .registers
                .set_flags($crate::flags::compute_flags_u16(
                    machine.registers.flags(),
                    result,
                    current_cf,
                    of,
                    af,
                ));

            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! dec_reg16 {
    ($name:ident, $get:ident, $set:ident, $opcode:literal) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let mut bytes = prev.to_vec();
            bytes.push($opcode);

            let old = machine.registers.$get();
            let result = old.wrapping_sub(1);
            let af = (old & 0x0F) == 0;
            let of = old == 0x8000;
            let current_cf = (machine.registers.flags() & $crate::flags::CF) != 0;

            machine.registers.$set(result);
            machine
                .registers
                .set_flags($crate::flags::compute_flags_u16(
                    machine.registers.flags(),
                    result,
                    current_cf,
                    of,
                    af,
                ));

            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! mov_reg8_imm8 {
    ($name:ident, $set:ident, $opcode:literal) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let mut bytes = prev.to_vec();
            let imm = machine.read_instr_u8(machine.registers.ip());
            bytes.push(imm);
            machine.log_instruction(csip, &bytes).ok();
            machine.registers.$set(imm);
            machine.registers.step(None);
        }
    };
}

/*#[macro_export]
macro_rules! push_reg16 {
    ($name:ident, $get:ident) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();
            machine
                .registers
                .set_sp(machine.registers.sp().wrapping_sub(2));
            machine.write_u16(
                machine.registers.ss(),
                machine.registers.sp(),
                machine.registers.$get(),
            );
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! pop_reg16 {
    ($name:ident, $set:ident) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();
            let reg = machine.read_u16(machine.registers.ss(), machine.registers.sp());
            machine
                .registers
                .set_sp(machine.registers.sp().wrapping_add(2));
            machine.registers.$set(reg);
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}*/

#[macro_export]
macro_rules! push_reg16 {
    ($name:ident, $reg:ident) => {
        pub(crate) fn $name(machine: &mut DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();
            let ss_base = (machine.registers.ss() as u32) << 4;
            let sp = machine.registers.sp().wrapping_sub(2);
            machine.registers.set_sp(sp);
            machine.write_phys_u16(ss_base.wrapping_add(sp as u32), machine.registers.$reg());
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! pop_reg16 {
    ($name:ident, $set_reg:ident) => {
        pub(crate) fn $name(machine: &mut DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();
            let ss_base = (machine.registers.ss() as u32) << 4;
            let sp = machine.registers.sp();
            let val = machine.read_phys_u16(ss_base.wrapping_add(sp as u32));
            machine.registers.set_sp(sp.wrapping_add(2));
            machine.registers.$set_reg(val);
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! xchg_ax_reg16 {
    ($name:ident, $get:ident, $set:ident) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();
            let ax = machine.registers.ax();
            let other = machine.registers.$get();
            machine.registers.set_ax(other);
            machine.registers.$set(ax);
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! xchg_eax_reg32 {
    ($name:ident, $get:ident, $set:ident) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();
            let eax = machine.registers.eax();
            let other = machine.registers.$get();
            machine.registers.set_eax(other);
            machine.registers.$set(eax);
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! dispatch_op32 {
    ($machine:expr, $op32:expr, $op16:expr) => {
        if $machine.has_operand_size_prefix {
            $op32
        } else {
            $op16
        }
    };
}

#[macro_export]
macro_rules! rep_movs_video_opt {
    ($machine:expr, $full_bytes:expr, $step:path, $elem_size:expr, $threshold:expr) => {
        if $machine.has_rep_prefix {
            // Проверка условий для быстрой видеокопии
            if $machine.video.mode == video::VideoMode::Mode13h
                && $machine.registers.es() == 0xA000
                && $machine.registers.ds() == 0xA000
                && $machine.registers.cx() > $threshold
            {
                let si = $machine.registers.si() as usize;
                let di = $machine.registers.di() as usize;
                let cx = $machine.registers.cx() as usize;
                let df = ($machine.registers.flags() & flags::DF) != 0;
                let video_size = 320 * 200 / $elem_size;

                let out_of_bounds = if df {
                    si / $elem_size < cx.saturating_sub(1)
                        || di / $elem_size < cx.saturating_sub(1)
                } else {
                    si / $elem_size + cx > video_size
                        || di / $elem_size + cx > video_size
                };

                if out_of_bounds {
                    // fallback на стандартный REP
                    while $machine.registers.cx() != 0 {
                        $step($machine, $full_bytes);
                        $machine.registers.set_cx($machine.registers.cx().wrapping_sub(1));
                    }
                } else if let Some(fb) = $machine.video.framebuffer.as_mut() {
                    if df {
                        // DF=1: стандартный цикл из-за сложности обратного копирования
                        while $machine.registers.cx() != 0 {
                            $step($machine, $full_bytes);
                            $machine.registers.set_cx($machine.registers.cx().wrapping_sub(1));
                        }
                    } else {
                        // DF=0: оптимизированное копирование
                        if $elem_size == 1 {
                            // Байтовый случай
                            if di > si && di < si + cx {
                                // Перекрытие (dst внутри src) — копируем назад
                                for i in (0..cx).rev() {
                                    fb.data[di + i] = fb.data[si + i];
                                }
                            } else {
                                // Прямое копирование
                                for i in 0..cx {
                                    fb.data[di + i] = fb.data[si + i];
                                }
                            }
                            $machine.registers.set_si((si + cx) as u16);
                            $machine.registers.set_di((di + cx) as u16);
                        } else {
                            // Словный случай ($elem_size == 2)
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
                            $machine.registers.set_si((si + cx * 2) as u16);
                            $machine.registers.set_di((di + cx * 2) as u16);
                        }
                        $machine.video.dirty = true;
                        $machine.registers.set_cx(0);
                    }
                } else {
                    // Нет framebuffer – fallback
                    while $machine.registers.cx() != 0 {
                        $step($machine, $full_bytes);
                        $machine.registers.set_cx($machine.registers.cx().wrapping_sub(1));
                    }
                }
            } else {
                // Не подходит под быстрый путь – стандартный REP
                while $machine.registers.cx() != 0 {
                    $step($machine, $full_bytes);
                    $machine.registers.set_cx($machine.registers.cx().wrapping_sub(1));
                }
            }
        } else {
            // Без REP – однократный вызов
            $step($machine, $full_bytes);
        }
    };
}

#[macro_export]
macro_rules! rep_stos_video_opt {
    ($machine:expr, $full_bytes:expr, $step:path) => {
        if $machine.has_rep_prefix {
            // Логирование (как в оригинале для STOSB)

            log::trace!(
                "REP STOS start: DI={:#04x}, CX={:#04x}, DF={}",
                $machine.registers.di(),
                $machine.registers.cx(),
                ($machine.registers.flags() & flags::DF) != 0
            );
            // Проверка условий для быстрой заливки экрана
            if $machine.video.mode == video::VideoMode::Mode13h
                && $machine.registers.es() == 0xA000
                && $machine.registers.di() == 0
                && $machine.registers.cx() == 320 * 200 / (if $step == mov::stosb { 1 } else { 2 })
            {
                // Быстрая заливка всего экрана одним цветом
                if let Some(fb) = $machine.video.framebuffer.as_mut() {
                    let color = $machine.registers.al();
                    for i in 0..(320 * 200) {
                        fb.data[i] = color;
                    }
                    $machine.video.dirty = true;
                    $machine.registers.set_cx(0);
                    $machine.registers.set_di(64000); // 320*200 = 64000 байт
                }
            } else {
                // Стандартный REP
                while $machine.registers.cx() != 0 {
                    $step($machine, $full_bytes);
                    $machine
                        .registers
                        .set_cx($machine.registers.cx().wrapping_sub(1));
                }
            }
        } else {
            $step($machine, $full_bytes);
        }
    };
}
