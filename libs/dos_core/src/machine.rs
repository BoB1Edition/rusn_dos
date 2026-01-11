use std::{fs::File, io::Write};

use log::{error, info, warn};

use crate::{consts::SEGMENT_SIZE, instructions::{self, stack}};

#[derive(Debug)]
pub struct DosMachine {
    pub memory: Box<[u8]>,
    pub registers: crate::registers::Registers,
    pub halted: bool,
    pub logfile: File,
}

impl DosMachine {
    fn compute_flags(result: u8, cf: bool, of: bool, af: bool) -> u16 {
        let mut flags = 0u16;
        // ZF, SF, PF — общие для всех
        if result == 0 {
            flags |= 1 << 6;
        } // ZF
        if (result & 0x80) != 0 {
            flags |= 1 << 7;
        } // SF
        if result.count_ones() % 2 == 0 {
            flags |= 1 << 2;
        } // PF

        // CF, OF, AF — зависят от операции
        if cf {
            flags |= 1 << 0;
        }
        if of {
            flags |= 1 << 11;
        }
        if af {
            flags |= 1 << 4;
        }

        flags
    }

    fn compute_logical_flags(result: u8) -> u16 {
        // Логические операции: CF=0, OF=0
        let mut flags = 0u16;
        if result == 0 {
            flags |= 1 << 6;
        } // ZF
        if (result & 0x80) != 0 {
            flags |= 1 << 7;
        } // SF
        if result.count_ones() % 2 == 0 {
            flags |= 1 << 2;
        } // PF
        // CF=0, OF=0 — по умолчанию
        flags
    }
    pub fn log_instruction(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        let hex_bytes: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
        writeln!(self.logfile, "{}", hex_bytes.join(" "))
    }

    fn execute0f(&mut self, opcode: u8, prev: &[u8]) {
        match opcode {
            0xB7 => {
                let modrm = self.read_u8(self.registers.cs(), self.registers.ip());
                let mut bytes = Vec::with_capacity(prev.len() + 2);
                bytes.extend_from_slice(prev);
                bytes.push(opcode);
                bytes.push(modrm);
                let _ = self.log_instruction(&bytes.as_slice());
                if (modrm & 0xC0) != 0xC0 {
                    error!("Memory operand in MOV r/m16, Sreg not supported yet execute0f");
                    self.halted = true;
                    return;
                }
                let sreg_field = (modrm >> 3) & 0x7; // какой сегментный регистр читать
                let dst_reg = modrm & 0x7; // куда записать
                let src_val = match sreg_field {
                    0 => self.registers.ax(),
                    1 => self.registers.cx(),
                    2 => self.registers.dx(),
                    3 => self.registers.bx(),
                    4 => self.registers.sp(),
                    5 => self.registers.bp(),
                    6 => self.registers.si(),
                    7 => self.registers.di(),
                    _ => unreachable!(),
                };
                match dst_reg {
                    0 => self.registers.set_eax(src_val as u32),
                    _ => unreachable!(),
                }
            }
            _ => {
                error!(
                    "Unsupported opcode0f {:#02X} at CS:IP = {:#04x}:{:#04x}",
                    opcode, self.registers.cs(), self.registers.ip()
                );
                self.halted = true;
            }
        }
    }

    fn execute32(&mut self, opcode: u8, prev: &[u8]) {
        match opcode {
            0x0f => {
                let new_opcode = self.read_u8(self.registers.cs(), self.registers.ip());
                self.registers.step(None);
                let mut bytes = Vec::with_capacity(prev.len() + 1);
                bytes.extend_from_slice(prev);
                bytes.push(opcode);
                self.execute0f(new_opcode,&bytes);
            }
            _ => {
                error!(
                    "Unsupported opcode32 {:#02X} at CS:IP = {:#04x}:{:#04x}",
                    opcode, self.registers.cs(), self.registers.ip()
                );
                self.halted = true;
            }
        };
    }

    fn execute(&mut self, opcode: u8) {
        match opcode {
            0xB4 => {
                let imm = self.read_u8(self.registers.cs(), self.registers.ip());
                let _ = self.log_instruction(&[opcode, imm]);
                self.registers.set_ah(imm);

                self.registers.step(None);
            }
            0xB8 => {
                let imm = self.read_u16(self.registers.cs(), self.registers.ip());
                let bytes = [opcode, (imm & 0xFF) as u8, ((imm >> 8) & 0xFF) as u8];
                let _ = self.log_instruction(&bytes);
                self.registers.set_ax(imm);
                self.registers.step(Some(2));
            }
            0xBA => {
                let imm = self.read_u16(self.registers.cs(), self.registers.ip());
                let bytes = [opcode, (imm & 0xFF) as u8, ((imm >> 8) & 0xFF) as u8];
                let _ = self.log_instruction(&bytes);
                self.registers.set_dx(imm);
                self.registers.step(Some(2));
            }
            0x1f => {
                let _ = self.log_instruction(&[opcode]);
                let ds = self.read_u16(self.registers.ss(), self.registers.sp());
                self.registers.set_sp(self.registers.sp().wrapping_add(2));
                self.registers.set_ds(ds);
            }

            0x58 => {
                let _ = self.log_instruction(&[opcode]);
                let ax = self.read_u16(self.registers.ss(), self.registers.sp());
                self.registers.set_sp(self.registers.sp().wrapping_add(2));
                self.registers.set_ax(ax);
            }

            0x53 => {
                let _ = self.log_instruction(&[opcode]);
                self.registers.set_sp(self.registers.sp().wrapping_sub(2));
                self.write_u16(self.registers.ss(), self.registers.sp(), self.registers.bx());
            }

            0x0E => {
                let _ = self.log_instruction(&[opcode]);
                self.registers.set_sp(self.registers.sp().wrapping_sub(2));
                self.write_u16(self.registers.ss(), self.registers.sp(), self.registers.cs());
            }
            0x50 => {
                let _ = self.log_instruction(&[opcode]);
                stack::push_ax(self);
            }
            0x9C => {
                let _ = self.log_instruction(&[opcode]);
                self.registers.set_sp(self.registers.sp().wrapping_sub(2));
                self.write_u16(self.registers.ss(), self.registers.sp(), self.registers.flags());
            }

            0x66 => {
                let new_opcode = self.read_u8(self.registers.cs(), self.registers.ip());
                self.registers.step(None);
                self.execute32(new_opcode, &[opcode]);
            }

            0x8C => {
                let modrm = self.read_u8(self.registers.cs(), self.registers.ip());
                let _ = self.log_instruction(&[opcode, modrm]);
                self.registers.step(None); // пропустить ModR/M

                if (modrm & 0xC0) != 0xC0 {
                    error!("Memory operand in MOV r/m16, Sreg not supported yet");
                    self.halted = true;
                    return;
                }

                let sreg_field = (modrm >> 3) & 0x7; // какой сегментный регистр читать
                let dst_reg = modrm & 0x7; // куда записать

                let sreg_value = match sreg_field {
                    0 => self.registers.es(),
                    1 => self.registers.cs(),
                    2 => self.registers.ss(),
                    3 => self.registers.ds(),
                    _ => {
                        warn!("Reserved segment register field: {}", sreg_field);
                        0
                    }
                };

                // Записываем в целевой 16-битный регистр
                match dst_reg {
                    0 => self.registers.set_ax(sreg_value), // AX
                    1 => self.registers.set_cx(sreg_value),    // CX
                    2 => self.registers.set_dx(sreg_value),    // DX
                    3 => self.registers.set_bx(sreg_value),    // BX
                    4 => self.registers.set_sp(sreg_value),    // SP
                    5 => self.registers.set_bp(sreg_value),    // BP
                    6 => self.registers.set_si(sreg_value),    // SI
                    7 => self.registers.set_di(sreg_value),    // DI
                    _ => unreachable!(),
                }
            }

            0xE8 => {
                let rel16 = self.read_u16(self.registers.cs(), self.registers.ip()) as i16;
                let bytes = [opcode, (rel16 & 0xFF) as u8, ((rel16 >> 8) & 0xFF) as u8];
                let _ = self.log_instruction(&bytes);
                let return_ip = self.registers.ip().wrapping_add(2);

                self.registers.set_sp(self.registers.sp().wrapping_sub(2));
                self.write_u16(self.registers.ss(), self.registers.sp(), return_ip);

                // Переход: IP = IP + 3 + rel16
                let new_ip = (return_ip as i32 + rel16 as i32) as u16;
                self.registers.set_ip(new_ip);
            }
            0xFC => {
                let _ = self.log_instruction(&[opcode]);
                self.registers.set_flags(self.registers.flags() & !0x0400);
            }
            0xC3 => {
                let _ = self.log_instruction(&[opcode]);
                let ip = self.read_u16(self.registers.ss(), self.registers.sp());
                self.registers.set_sp(self.registers.sp().wrapping_add(2));
                self.registers.set_ip(ip);
            }
            0x32 => {
                let modrm = self.read_u8(self.registers.cs(), self.registers.ip());
                let _ = self.log_instruction(&[opcode, modrm]);
                self.registers.step(None); // skip ModR/M

                let reg_dst = (modrm >> 3) & 0x7; // целевой регистр (r8)
                let rm_src = modrm & 0x7; // источник (r/m8)

                let r_src = self.get_registry_value(rm_src);
                match reg_dst {
                    0 => self.registers.set_al(self.registers.al() ^ r_src), // AL
                    1 => self.registers.set_bl(self.registers.bl() ^ r_src), // BL
                    2 => self.registers.set_cl(self.registers.cl() ^ r_src), // CL = 0
                    3 => self.registers.set_dl(self.registers.dl() ^ r_src), // DL = 0
                    4 => self.registers.set_ah(self.registers.ah() ^ r_src), // AH
                    5 => self.registers.set_bh(self.registers.bh() ^ r_src), // BH = 0
                    6 => self.registers.set_ch(self.registers.ch() ^ r_src), // CH = 0
                    7 => self.registers.set_dh(self.registers.dh() ^ r_src), // DH = 0
                    _ => unreachable!(),
                };
            }
            0x74 => {
                let rel8 = self.read_u8(self.registers.cs(), self.registers.ip()) as i8;
                let _ = self.log_instruction(&[opcode, rel8 as u8]);
                self.registers.step(None); // пропустить rel8

                if (self.registers.flags() & (1 << 6)) != 0 {
                    // ZF = 1?
                    // Выполняем переход
                    let new_ip = (self.registers.ip() as i32 + rel8 as i32) as u16;
                    self.registers.set_ip(new_ip);
                }
                // Если ZF = 0 — просто продолжаем выполнение
            }
            0x9D => {
                let _ = self.log_instruction(&[opcode]);
                let flags = self.read_u16(self.registers.ss(), self.registers.sp());
                self.registers.set_sp(self.registers.sp().wrapping_add(2));
                self.registers.set_flags(flags);
                //println!("flags: ");
                //self.print_4byte(self.registers.cs, self.registers.ip);
            }
            0x80 => {
                let modrm = self.read_u8(self.registers.cs(), self.registers.ip());
                let imm8 = self.read_u8(self.registers.cs(), self.registers.ip().wrapping_add(1));
                self.registers.step(Some(2)); // пропустить ModR/M + imm8
                let _ = self.log_instruction(&[opcode, modrm, imm8]);
                let mod_field = (modrm >> 6) & 0x3;
                let reg_field = (modrm >> 3) & 0x7;
                let rm_field = modrm & 0x7;

                match mod_field {
                    0b00 => self.group_x80_operation_memory(reg_field, rm_field, imm8),
                    0b01 => self.group_x80_operation_memory_1byte(reg_field, rm_field, imm8),
                    0b10 => self.group_x80_operation_memory_2byte(reg_field, rm_field, imm8),
                    0b11 => self.group_x80_operation_registry(reg_field, rm_field, imm8),
                    _ => unreachable!(),
                }

                /*self.print_4byte(self.registers.cs, self.registers.ip);
                self.halted = true;*/
            }
            0xCD => {
                // INT nn
                let vector = self.read_u8(self.registers.cs(), self.registers.ip());
                let _ = self.log_instruction(&[opcode, vector]);
                self.registers.step(None);
                match vector {
                    0x21 => self.handle_int21(),
                    0x20 => self.halted = true,
                    _ => {
                        warn!("Unsupported interrupt: INT {:#02X}", vector);
                        self.halted = true;
                    }
                }
            }
            0x00 => {
                let modrm = self.read_u8(self.registers.cs(), self.registers.ip());
                self.registers.step(None); // skip ModR/M
                let _ = self.log_instruction(&[opcode, modrm]);
                let reg_field = (modrm >> 3) & 0x7; // источник (r8)
                let rm_field = modrm & 0x7; // приёмник (r/m8)

                // Поддержим только регистровый режим (Mod=11)
                if (modrm & 0xC0) != 0xC0 {
                    error!("Memory operand in ADD r/m8, r8 not supported yet");
                    self.halted = true;
                    return;
                }

                let src_val = match reg_field {
                    0 => self.registers.al(),
                    1 => self.registers.cl(),
                    2 => self.registers.dl(),
                    3 => self.registers.bl(),
                    4 => self.registers.ah(),
                    5 => self.registers.ch(),
                    6 => self.registers.dh(),
                    7 => self.registers.bh(),
                    _ => unreachable!(),
                };

                let dst_val = match rm_field {
                    0 => self.registers.al(),
                    1 => self.registers.cl(),
                    2 => self.registers.dl(),
                    3 => self.registers.bl(),
                    4 => self.registers.ah(),
                    5 => self.registers.ch(),
                    6 => self.registers.dh(),
                    7 => self.registers.bh(),
                    _ => unreachable!(),
                };

                let res = (dst_val as u16).wrapping_add(src_val as u16);
                let result = res as u8;

                // Обновляем флаги
                let mut flags = self.registers.flags();
                flags &= !(1 << 0 | 1 << 2 | 1 << 4 | 1 << 6 | 1 << 7 | 1 << 11); // CF, PF, AF, ZF, SF, OF
                if res > 0xFF {
                    flags |= 1 << 0;
                } // CF
                if ((dst_val & 0x0F) + (src_val & 0x0F)) > 0x0F {
                    flags |= 1 << 4;
                } // AF
                if (((dst_val ^ src_val) & 0x80) == 0) && ((dst_val ^ result) & 0x80) != 0 {
                    flags |= 1 << 11;
                } // OF
                if result == 0 {
                    flags |= 1 << 6;
                } // ZF
                if (result & 0x80) != 0 {
                    flags |= 1 << 7;
                } // SF
                if result.count_ones() % 2 == 0 {
                    flags |= 1 << 2;
                } // PF

                match rm_field {
                    0 => self.registers.set_al(result),
                    1 => self.registers.set_cl(result),
                    2 => self.registers.set_dl(result),
                    3 => self.registers.set_bl(result),
                    4 => self.registers.set_ah(result),
                    5 => self.registers.set_ch(result),
                    6 => self.registers.set_dh(result),
                    7 => self.registers.set_bh(result),
                    _ => unreachable!(),
                }

                self.registers.set_flags(flags);
            }
            0xFA => {
                let _ = self.log_instruction(&[opcode]);
                self.registers.set_flags(self.registers.flags() & !0x0200);
                //self.registers.step(None);
            }
            0xFB => {
                let _ = self.log_instruction(&[opcode]);
                // STI: Set Interrupt Flag (bit 9)
                self.registers.set_flags(self.registers.flags() | 0x0200);
                //self.registers.step(None);
            }
            _ => {
                error!(
                    "Unsupported opcode {:#02X} at CS:IP = {:#04x}:{:#04x}",
                    opcode, self.registers.cs(), self.registers.ip()
                );
                self.halted = true;
            }
        }
    }
    pub fn run(&mut self) {
        //env_logger::init();
        while !self.halted {
            //println!("self.registers.ip: {}", self.registers.ip);
            let opcode = self.read_u8(self.registers.cs(), self.registers.ip());
            self.registers.step(None);
            self.execute(opcode);
        }
    }
    fn handle_int21(&mut self) {
        match self.registers.ah() {
            0x09 => {
                self.print_dos_string()
            },
            0x4C => {
                self.halted = true;
            },
            _ => panic!("Unsupported DOS call AH={:#02x}", self.registers.ah()),
        }
    }
    fn print_dos_string(&self) {
        let mut addr = self.registers.dx() as usize;
        let mut s = String::new();
        loop {
            if addr >= self.memory.len() {
                log::error!("string not contains '$'");
                return;
            }
            let byte = self.memory[addr];
            if byte == b'$' {
                break;
            }
            s.push(byte as char);
            addr += 1;
        }
        println!("{}", s);
    }
    #[inline(always)]
    fn read_u8(&self, segment: u16, offset: u16) -> u8 {
        let addr = (segment as usize) * 16 + (offset as usize);
        if addr < self.memory.len() {
            self.memory[addr]
        } else {
            error!("stack overflow: {}", addr);
            0xFF
        }
    }
    #[inline(always)]
    pub fn read_u16(&self, segment: u16, offset: u16) -> u16 {
        let lo = self.read_u8(segment, offset) as u16;
        let hi = self.read_u8(segment, offset.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }
    #[inline(always)]
    fn write_u8(&mut self, segment: u16, offset: u16, value: u8) {
        let addr = (segment as usize) * SEGMENT_SIZE + (offset as usize);
        if addr < self.memory.len() {
            self.memory[addr] = value;
        } else {
            log::error!("Memory write out of bounds: {:#x}", addr);
        }
    }
    #[inline(always)]
    pub fn write_u16(&mut self, segment: u16, offset: u16, value: u16) {
        self.write_u8(segment, offset, value as u8);
        self.write_u8(segment, offset.wrapping_add(1), (value >> 8) as u8);
    }

    fn print_4byte(&self, segment: u16, offset: u16) {
        for i in 0..10 {
            let op = self.read_u8(segment, offset + i);
            println!("op{i}: {op:#02X}")
        }
    }

    fn group_x80_operation_memory(&self, reg_field: u8, rm_field: u8, imm8: u8) {
        todo!("group_x80_operation_memory")
    }

    fn group_x80_operation_memory_1byte(&self, reg_field: u8, rm_field: u8, imm8: u8) {
        todo!("group_x80_operation_memory_1byte")
    }

    fn group_x80_operation_memory_2byte(&self, reg_field: u8, rm_field: u8, imm8: u8) {
        todo!("group_x80_operation_memory_2byte")
    }

    fn group_x80_operation_registry(&mut self, reg_field: u8, rm_field: u8, imm8: u8) {
        let src_val = self.get_registry_value(rm_field);
        let imm = imm8 as u8;

        // Все вычисления делаем в u16, чтобы ловить переносы
        let (result_u8, flags) = match reg_field {
            0 => {
                // ADD r8, imm8
                let res = src_val as u16 + imm as u16;
                let result = res as u8;
                let cf = (res >> 8) != 0;
                let af = ((src_val & 0x0F) + (imm & 0x0F)) > 0x0F;
                let of = (((src_val ^ imm) & 0x80) == 0) && ((src_val ^ result) & 0x80) != 0;
                (result, Self::compute_flags(result, cf, of, af))
            }
            1 => {
                // OR r8, imm8
                let result = src_val | imm;
                (result, Self::compute_logical_flags(result))
            }
            2 => {
                // ADC r8, imm8
                let carry_in = (self.registers.flags() & 1) != 0;
                let mut res = src_val as u16 + imm as u16;
                if carry_in {
                    res += 1;
                }
                let result = res as u8;
                let cf = (res >> 8) != 0;
                let af = ((src_val & 0x0F) + (imm & 0x0F) + if carry_in { 1 } else { 0 }) > 0x0F;
                let of = (((src_val ^ imm) & 0x80) == 0) && ((src_val ^ result) & 0x80) != 0;
                (result, Self::compute_flags(result, cf, of, af))
            }
            3 => {
                // SBB r8, imm8
                let borrow_in = (self.registers.flags() & 1) != 0;
                let mut res = src_val as u16;
                let subtrahend = imm as u16 + if borrow_in { 1 } else { 0 };
                let cf = res < subtrahend;
                res = res.wrapping_sub(subtrahend);
                let result = res as u8;
                let af = (src_val & 0x0F) < (imm & 0x0F) + if borrow_in { 1 } else { 0 };
                let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
                (result, Self::compute_flags(result, cf, of, af))
            }
            4 => {
                // AND r8, imm8
                let result = src_val & imm;
                (result, Self::compute_logical_flags(result))
            }
            5 => {
                // SUB r8, imm8
                let res = (src_val as u16).wrapping_sub(imm as u16);
                let result = res as u8;
                let cf = src_val < imm;
                let af = (src_val & 0x0F) < (imm & 0x0F);
                let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
                (result, Self::compute_flags(result, cf, of, af))
            }
            6 => {
                // XOR r8, imm8
                let result = src_val ^ imm;
                (result, Self::compute_logical_flags(result))
            }
            7 => {
                // CMP r8, imm8 — как SUB, но не сохраняем результат
                let res = (src_val as u16).wrapping_sub(imm as u16);
                let result = res as u8;
                let cf = src_val < imm;
                let af = (src_val & 0x0F) < (imm & 0x0F);
                let of = (((src_val ^ imm) & 0x80) != 0) && ((src_val ^ result) & 0x80) != 0;
                let flags = Self::compute_flags(result, cf, of, af);
                self.registers.set_flags(flags);
                return; // выход без записи результата
            }
            _ => unreachable!(),
        };

        // Записываем результат (если не CMP)
        match rm_field {
            0 => self.registers.set_al(result_u8),
            1 => self.registers.set_cl(result_u8),
            2 => self.registers.set_dl(result_u8),
            3 => self.registers.set_bl(result_u8),
            4 => self.registers.set_ah(result_u8),
            5 => self.registers.set_ch(result_u8),
            6 => self.registers.set_dh(result_u8),
            7 => self.registers.set_bh(result_u8),
            _ => unreachable!(),
        }

        self.registers.set_flags(flags);
    }

    fn get_registry_value(&self, rm_src: u8) -> u8 {
        match rm_src {
            0 => self.registers.al(), // AL
            1 => self.registers.cl(), // CL
            2 => self.registers.dl(), // DL
            3 => self.registers.bl(), // BL
            4 => self.registers.ah(), // AH
            5 => self.registers.ch(), // CH
            6 => self.registers.dh(), // DH
            7 => self.registers.bh(), // BH
            _ => unreachable!(),
        }
    }
}