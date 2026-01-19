use std::{fs::File, io::Write};

use log::error;

use crate::{
    consts::SEGMENT_SIZE,
    instructions::{alu, alu32, control, mov, mov32, stack, system},
};

#[derive(Debug)]
pub struct DosMachine {
    pub memory: Box<[u8]>,
    pub registers: crate::registers::Registers,
    pub halted: bool,
    pub logfile: File,
    pub has_address_size_prefix: bool,
    pub has_operand_size_prefix: bool,
}

impl DosMachine {
    #[inline]
    pub fn read_reg8(&self, reg: u8) -> u8 {
        match reg {
            0 => self.registers.al(),
            1 => self.registers.cl(),
            2 => self.registers.dl(),
            3 => self.registers.bl(),
            4 => self.registers.ah(),
            5 => self.registers.ch(),
            6 => self.registers.dh(),
            7 => self.registers.bh(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn write_reg8(&mut self, reg: u8, value: u8) {
        match reg {
            0 => self.registers.set_al(value),
            1 => self.registers.set_cl(value),
            2 => self.registers.set_dl(value),
            3 => self.registers.set_bl(value),
            4 => self.registers.set_ah(value),
            5 => self.registers.set_ch(value),
            6 => self.registers.set_dh(value),
            7 => self.registers.set_bh(value),
            _ => unreachable!(),
        }
    }
    #[inline]
    pub fn read_reg16(&self, reg: u8) -> u16 {
        match reg {
            0 => self.registers.ax(),
            1 => self.registers.cx(),
            2 => self.registers.dx(),
            3 => self.registers.bx(),
            4 => self.registers.sp(),
            5 => self.registers.bp(),
            6 => self.registers.si(),
            7 => self.registers.di(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn write_reg16(&mut self, reg: u8, value: u16) {
        match reg {
            0 => self.registers.set_ax(value),
            1 => self.registers.set_cx(value),
            2 => self.registers.set_dx(value),
            3 => self.registers.set_bx(value),
            4 => self.registers.set_sp(value),
            5 => self.registers.set_bp(value),
            6 => self.registers.set_si(value),
            7 => self.registers.set_di(value),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn read_sreg(&self, sreg: u8) -> u16 {
        match sreg {
            0 => self.registers.es(),
            1 => self.registers.cs(),
            2 => self.registers.ss(),
            3 => self.registers.ds(),
            _ => 0, // зарезервировано
        }
    }

    #[inline]
    pub fn read_reg32(&self, reg: u8) -> u32 {
        match reg {
            0 => self.registers.eax(),
            1 => todo!("self.registers.ecx(),"),
            2 => todo!("self.registers.edx(),"),
            3 => self.registers.ebx(),
            4 => todo!("self.registers.esp(),"),
            5 => todo!("self.registers.ebp(),"),
            6 => todo!("self.registers.esi(),"),
            7 => todo!("self.registers.edi(),"),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn write_reg32(&mut self, reg: u8, value: u32) {
        match reg {
            0 => self.registers.set_eax(value),
            1 => todo!("self.registers.set_ecx(value)"),
            2 => todo!("self.registers.set_edx(value)"),
            3 => todo!("self.registers.set_ebx(value)"),
            4 => todo!("self.registers.set_esp(value)"),
            5 => todo!("self.registers.set_ebp(value)"),
            6 => todo!("self.registers.set_esi(value)"),
            7 => todo!("self.registers.set_edi(value)"),
            _ => unreachable!(),
        }
    }

    pub fn compute_flags(result: u8, cf: bool, of: bool, af: bool) -> u16 {
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

    pub fn compute_logical_flags(result: u8) -> u16 {
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
                self.registers.step(None);
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
                    opcode,
                    self.registers.cs(),
                    self.registers.ip()
                );
                self.halted = true;
            }
        }
    }

    pub fn print_error_exit_address(&mut self, opcode: u8) {
        let bit_depth = if self.has_address_size_prefix {
            "address32"
        } else {
            "address32"
        }
        .to_string();
        error!(
            "Unsupported {bit_depth} {:#02X} at CS:IP = {:#04x}:{:#04x}",
            opcode,
            self.registers.cs(),
            self.registers.ip()
        );
        self.halted = true;
    }

    fn print_error_exit(&mut self, opcode: u8) {
        let bit_depth = if self.has_operand_size_prefix {
            "opcode32"
        } else {
            "opcode"
        }
        .to_string();
        error!(
            "Unsupported {bit_depth} {:#02X} at CS:IP = {:#04x}:{:#04x}",
            opcode,
            self.registers.cs(),
            self.registers.ip()
        );
        self.halted = true;
    }

    fn execute(&mut self, opcode: u8) {
        match opcode {
            0xB4 => {
                if !self.has_address_size_prefix {
                    mov::mov_ah(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x67 => {
                self.has_address_size_prefix = true;
            }
            0x0f => {
                if self.has_operand_size_prefix {
                    let new_opcode = self.read_u8(self.registers.cs(), self.registers.ip());
                    self.registers.step(None);
                    self.execute0f(new_opcode, &[0x66, opcode]);
                    self.has_operand_size_prefix = !self.has_operand_size_prefix;
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xB8 => {
                if !self.has_operand_size_prefix {
                    mov::mov_ax(self, &[opcode]);
                } else {
                    mov32::mov_eax_data(self, &[0x66, opcode]);
                }
            }
            0xBA => {
                if !self.has_operand_size_prefix {
                    mov::mov_dx(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xBB => {
                if self.has_operand_size_prefix {
                    mov32::mov_ebx_data(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x1f => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    stack::pop_ds(self);
                } else {
                    self.print_error_exit(opcode);
                }
            }

            0x58 => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    stack::pop_ax(self);
                } else {
                    self.print_error_exit(opcode);
                }
            }

            0x53 => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    stack::push_bx(self);
                } else {
                    self.print_error_exit(opcode);
                }
            }

            0x0E => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    stack::push_cs(self);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x50 => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    stack::push_ax(self);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x9C => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    stack::pushf(self);
                } else {
                    self.print_error_exit(opcode);
                }
            }

            0x66 => {
                self.has_operand_size_prefix = true;
            }
            0xA3 => {
                if self.has_operand_size_prefix {
                    if self.has_address_size_prefix {
                        mov32::mov_address_eax(self, &[0x66, 0x67, opcode]);
                        self.has_operand_size_prefix = !self.has_operand_size_prefix;
                    } else {
                        mov32::mov_address_eax(self, &[0x66, 0x67, opcode]);
                    }
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xC1 => {
                if self.has_operand_size_prefix {
                    alu32::shift_group_c1(self, &[0x66, opcode]);
                    self.has_operand_size_prefix = !self.has_operand_size_prefix;
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x8C => {
                if !self.has_operand_size_prefix {
                    mov::mov(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }

            0xE8 => {
                if !self.has_operand_size_prefix {
                    control::call(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xFC => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    self.registers.set_flags(self.registers.flags() & !0x0400);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xC3 => {
                if !self.has_operand_size_prefix {
                    control::retn(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x32 => {
                if !self.has_operand_size_prefix {
                    alu::xor(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x74 => {
                if !self.has_operand_size_prefix {
                    control::jz(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x9D => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    stack::popf(self);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x80 => {
                if !self.has_operand_size_prefix {
                    alu::group_x80(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xCD => {
                if !self.has_operand_size_prefix {
                    system::int(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x00 => {
                if !self.has_operand_size_prefix {
                    alu::add(self, &[opcode]);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xFA => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    self.registers.set_flags(self.registers.flags() & !0x0200);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xFB => {
                if !self.has_operand_size_prefix {
                    let _ = self.log_instruction(&[opcode]);
                    self.registers.set_flags(self.registers.flags() | 0x0200);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x09 => {
                if self.has_operand_size_prefix {
                    if self.has_address_size_prefix {
                        alu32::or(self, &[0x66, 0x67, opcode]);
                    } else {
                        alu32::or(self, &[0x66, opcode]);
                    }
                }
            }
            _ => {
                self.print_error_exit(opcode);
            }
        }
    }
    pub fn run(&mut self) {
        while !self.halted {
            let opcode = self.read_u8(self.registers.cs(), self.registers.ip());
            self.registers.step(None);
            self.execute(opcode);
        }
    }
    pub fn handle_int21(&mut self) {
        match self.registers.ah() {
            0x09 => self.print_dos_string(),
            0x4C => {
                self.halted = true;
            }
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
    pub fn read_u8(&self, segment: u16, offset: u16) -> u8 {
        let addr = (segment as usize) * 16 + (offset as usize);
        if addr < self.memory.len() {
            self.memory[addr]
        } else {
            error!("stack overflow: {}", addr);
            0xFF
        }
    }

    #[inline(always)]
    pub fn read_phys_u8(&self, addr: u32) -> u8 {
        let addr_20bit = (addr & 0xFFFFF) as usize; // 20-битный wrap-around
        if addr_20bit < self.memory.len() {
            self.memory[addr_20bit]
        } else {
            0xFF // или panic, или лог ошибки
        }
    }

    #[inline(always)]
    pub fn read_phys_u16(&self, addr: u32) -> u16 {
        let lo = self.read_phys_u8(addr) as u16;
        let hi = self.read_phys_u8(addr + 1) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    pub fn read_phys_u32(&self, addr: u32) -> u32 {
        let lo = self.read_phys_u16(addr) as u32;
        let hi = self.read_phys_u16(addr + 2) as u32;
        lo | (hi << 16)
    }
    #[inline(always)]
    pub fn read_u16(&self, segment: u16, offset: u16) -> u16 {
        let lo = self.read_u8(segment, offset) as u16;
        let hi = self.read_u8(segment, offset.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    /*pub fn read_u32(&self, segment: u16, offset: u16) -> u32 {
        let lo = self.read_u16(segment, offset) as u32;
        let hi = self.read_u16(segment, offset.wrapping_add(2)) as u32;
        lo | (hi << 16)
    }*/
    pub fn read_u32(&self, segment: u16, offset: u16) -> u32 {
        let b0 = self.read_u8(segment, offset) as u32;
        let b1 = self.read_u8(segment, offset.wrapping_add(1)) as u32;
        let b2 = self.read_u8(segment, offset.wrapping_add(2)) as u32;
        let b3 = self.read_u8(segment, offset.wrapping_add(3)) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
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

    #[inline(always)]
    pub fn write_u32(&mut self, segment: u16, offset: u16, value: u32) {
        self.write_u16(segment, offset, value as u16);
        self.write_u16(segment, offset.wrapping_add(2), (value >> 16) as u16);
    }

    fn print_4byte(&self, segment: u16, offset: u16) {
        for i in 0..10 {
            let op = self.read_u8(segment, offset + i);
            println!("op{i}: {op:#02X}")
        }
    }

    #[inline(always)]
    pub fn write_phys_u32(&mut self, addr: u32, value: u32) {
        let addr_20bit = addr & 0xFFFFF;
        self.write_phys_u16(addr_20bit, value as u16);
        self.write_phys_u16(addr_20bit + 2, (value >> 16) as u16);
    }

    #[inline(always)]
    pub fn write_phys_u16(&mut self, addr: u32, value: u16) {
        let addr = addr & 0xFFFFF;
        self.write_phys_u8(addr, value as u8);
        self.write_phys_u8(addr + 1, (value >> 8) as u8);
    }

    #[inline(always)]
    fn write_phys_u8(&mut self, addr: u32, value: u8) {
        let idx = (addr & 0xFFFFF) as usize;
        if idx < self.memory.len() {
            self.memory[idx] = value;
        }
    }
}
