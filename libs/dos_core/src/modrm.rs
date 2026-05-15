// Ver: 1

use crate::DosMachine;

#[derive(Debug, Clone, Copy)]
pub struct ModRm {
    pub mod_field: u8,
    pub reg_field: u8,
    pub rm_field: u8,
}

impl ModRm {
    #[inline]
    pub fn from_byte(byte: u8) -> Self {
        Self {
            mod_field: (byte >> 6) & 0x3,
            reg_field: (byte >> 3) & 0x7,
            rm_field: byte & 0x7,
        }
    }

    #[inline]
    pub fn is_register_mode(&self) -> bool {
        self.mod_field == 0b11
    }

    #[inline]
    pub fn resolve_address(
        &self,
        machine: &mut DosMachine,
        addr32_mode: bool,
        bytes: &mut Vec<u8>,
    ) -> Option<u32> {
        if self.is_register_mode() {
            return None;
        }

        let segment = machine.override_segment.unwrap_or(machine.registers.ds());

        if addr32_mode {
            // === 32-битная адресация ===
            if self.rm_field == 4 {
                // Присутствует SIB-байт
                let sib_byte = machine.read_instr_u8(machine.registers.ip());
                machine.registers.step(None);
                bytes.push(sib_byte);

                let scale = (sib_byte >> 6) & 0x3; // множитель 1,2,4,8
                let index_field = (sib_byte >> 3) & 0x7; // регистр-индекс
                let base_field = sib_byte & 0x7; // базовый регистр

                // Вычисляем масштабированный индекс
                let index_val = match index_field {
                    4 => 0, // индекс не используется (ESP как индекс запрещён)
                    0 => machine.registers.eax(),
                    1 => machine.registers.ecx(),
                    2 => machine.registers.edx(),
                    3 => machine.registers.ebx(),
                    5 => machine.registers.ebp(),
                    6 => machine.registers.esi(),
                    7 => machine.registers.edi(),
                    _ => unreachable!(),
                };
                let scaled_index = index_val << scale;

                // Специальный случай: base = EBP и mod = 0 → [scaled_index + disp32]
                if base_field == 5 && self.mod_field == 0 {
                    let disp = machine.read_instr_u32(machine.registers.ip());
                    machine.registers.step(Some(4));
                    bytes.extend_from_slice(&disp.to_le_bytes());
                    let effective = scaled_index.wrapping_add(disp);
                    // База EBP → сегмент SS по умолчанию
                    let seg = machine.override_segment.unwrap_or(machine.registers.ss());
                    return Some(((seg as u32) << 4).wrapping_add(effective));
                }

                // Обычный базовый регистр
                let base_val = match base_field {
                    0 => machine.registers.eax(),
                    1 => machine.registers.ecx(),
                    2 => machine.registers.edx(),
                    3 => machine.registers.ebx(),
                    4 => machine.registers.esp(),
                    5 => machine.registers.ebp(), // mod != 0
                    6 => machine.registers.esi(),
                    7 => machine.registers.edi(),
                    _ => unreachable!(),
                };

                // Displacement (зависит от mod)
                let disp = match self.mod_field {
                    0 => 0,
                    1 => {
                        let d = machine.read_instr_u8(machine.registers.ip()) as i8 as i32;
                        machine.registers.step(None);
                        bytes.push(d as u8);
                        d
                    }
                    2 => {
                        let d = machine.read_instr_u32(machine.registers.ip()) as i32;
                        machine.registers.step(Some(4));
                        bytes.extend_from_slice(&d.to_le_bytes());
                        d
                    }
                    _ => unreachable!(),
                };

                let effective = (base_val as i32)
                    .wrapping_add(scaled_index as i32)
                    .wrapping_add(disp) as u32;

                // Сегмент по умолчанию: SS для EBP/ESP, иначе DS
                let default_seg = if base_field == 5 || base_field == 4 {
                    machine.registers.ss()
                } else {
                    machine.registers.ds()
                };
                let seg = machine.override_segment.unwrap_or(default_seg);
                return Some(((seg as u32) << 4).wrapping_add(effective));
            } else {
                // Без SIB — обычные регистры
                let base = match self.rm_field {
                    0 => machine.registers.eax(),
                    1 => machine.registers.ecx(),
                    2 => machine.registers.edx(),
                    3 => machine.registers.ebx(),
                    4 => machine.registers.esp(),
                    5 => {
                        if self.mod_field == 0 {
                            // [disp32]
                            let disp = machine.read_instr_u32(machine.registers.ip());
                            machine.registers.step(Some(4));
                            let seg = machine.override_segment.unwrap_or(machine.registers.ds());
                            return Some(((seg as u32) << 4).wrapping_add(disp));
                        } else {
                            machine.registers.ebp()
                        }
                    }
                    6 => machine.registers.esi(),
                    7 => machine.registers.edi(),
                    _ => unreachable!(),
                };

                let disp = match self.mod_field {
                    0 => 0,
                    1 => {
                        let d = machine.read_instr_u8(machine.registers.ip()) as i8 as i32;
                        machine.registers.step(None);
                        bytes.push(d as u8);
                        d
                    }
                    2 => {
                        let d = machine.read_instr_u32(machine.registers.ip()) as i32;
                        machine.registers.step(Some(4));
                        bytes.extend_from_slice(&d.to_le_bytes());
                        d
                    }
                    _ => unreachable!(),
                };

                let effective = (base as i32).wrapping_add(disp) as u32;
                let default_seg = if self.rm_field == 5 || self.rm_field == 4 {
                    machine.registers.ss()
                } else {
                    machine.registers.ds()
                };
                let seg = machine.override_segment.unwrap_or(default_seg);
                return Some(((seg as u32) << 4).wrapping_add(effective));
            }
        } else {
            // === 16-битная адресация (исправлен сегмент для DI) ===
            let (base, index) = match self.rm_field {
                0 => (machine.registers.bx(), machine.registers.si()),
                1 => (machine.registers.bx(), machine.registers.di()),
                2 => (machine.registers.bp(), machine.registers.si()),
                3 => (machine.registers.bp(), machine.registers.di()),
                4 => (0, machine.registers.si()),
                5 => (0, machine.registers.di()),
                6 => {
                    if self.mod_field == 0 {
                        // [disp16]
                        let disp = machine.read_instr_u16(machine.registers.ip());
                        machine.registers.step(Some(2));
                        bytes.extend_from_slice(&disp.to_le_bytes());
                        let seg = machine.override_segment.unwrap_or(machine.registers.ds());
                        return Some(((seg as u32) << 4).wrapping_add(disp as u32));
                    } else {
                        (machine.registers.bp(), 0)
                    }
                }
                7 => (machine.registers.bx(), 0),
                _ => unreachable!(),
            };

            let disp = match self.mod_field {
                0 => 0,
                1 => {
                    let d = machine.read_instr_u8(machine.registers.ip()) as i8 as i16;
                    machine.registers.step(None);
                    bytes.push(d as u8);
                    d as i32
                }
                2 => {
                    let d = machine.read_instr_u16(machine.registers.ip()) as i16;
                    machine.registers.step(Some(2));
                    bytes.extend_from_slice(&d.to_le_bytes());
                    d as i32
                }
                _ => unreachable!(),
            };

            let effective = (base as i32 + index as i32 + disp) as u32;

            let uses_bp_or_sp = self.rm_field == 2
                || self.rm_field == 3
                || (self.rm_field == 6 && self.mod_field != 0);
            let effective_segment = if uses_bp_or_sp {
                machine.registers.ss()
            } else {
                segment
            };

            Some(((effective_segment as u32) << 4).wrapping_add(effective))
        }
    }
}
