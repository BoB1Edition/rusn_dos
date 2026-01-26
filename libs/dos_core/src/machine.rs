use std::{fs::File, io::Write};

use log::error;

use crate::{
    consts::SEGMENT_SIZE,
    instructions::{alu, alu32, control, control32, extended, extended32, mov, mov32, stack, system},
};

#[derive(Debug)]
pub struct DosMachine {
    pub memory: Box<[u8]>,
    pub registers: crate::registers::Registers,
    pub halted: bool,
    pub logfile: File,
    pub has_address_size_prefix: bool,
    pub has_operand_size_prefix: bool,
    pub has_extended_prefix: bool,
    pub override_segment: Option<u16>,
    pub opcode_override_segment: Option<u8>,
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
            3 => self.registers.set_ebx(value),
            4 => todo!("self.registers.set_esp(value)"),
            5 => todo!("self.registers.set_ebp(value)"),
            6 => todo!("self.registers.set_esi(value)"),
            7 => todo!("self.registers.set_edi(value)"),
            _ => unreachable!(),
        }
    }

    /*pub fn compute_flags(result: u8, cf: bool, of: bool, af: bool) -> u16 {
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
    }*/

    #[inline]
    fn compute_flags_impl(value: u32, width_bits: u32, cf: bool, of: bool, af: bool) -> u16 {
        let mut flags = 0u16;
        if value == 0 {
            flags |= 1 << 6; // ZF
        }
        let sign_bit = 1u32 << (width_bits - 1);
        if (value & sign_bit) != 0 {
            flags |= 1 << 7; // SF
        }
        if (value as u8).count_ones() % 2 == 0 {
            flags |= 1 << 2; // PF
        }
        if cf {
            flags |= 1 << 0;
        } // CF
        if of {
            flags |= 1 << 11;
        } // OF
        if af {
            flags |= 1 << 4;
        } // AF
        flags
    }

    #[inline]
    pub fn compute_flags_u8(result: u8, cf: bool, of: bool, af: bool) -> u16 {
        Self::compute_flags_impl(result as u32, 8, cf, of, af)
    }

    #[inline]
    pub fn compute_flags_u16(result: u16, cf: bool, of: bool, af: bool) -> u16 {
        Self::compute_flags_impl(result as u32, 16, cf, of, af)
    }

    #[inline]
    pub fn compute_flags_u32(result: u32, cf: bool, of: bool, af: bool) -> u16 {
        Self::compute_flags_impl(result, 32, cf, of, af)
    }

    // Логические операции (CF=0, OF=0)
    #[inline]
    pub fn compute_logical_flags_u8(result: u8) -> u16 {
        Self::compute_flags_u8(result, false, false, false)
    }

    #[inline]
    pub fn compute_logical_flags_u16(result: u16) -> u16 {
        Self::compute_flags_u16(result, false, false, false)
    }

    #[inline]
    pub fn compute_logical_flags_u32(result: u32) -> u16 {
        Self::compute_flags_u32(result, false, false, false)
    }
    pub fn log_instruction(&mut self, csip: [u16; 2], bytes: &[u8]) -> std::io::Result<()> {
        let hex_bytes: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
        writeln!(
            self.logfile,
            "{:#04x}:{:#04x}: {}",
            csip[0],
            csip[1],
            hex_bytes.join(" ")
        )
    }

    fn execute_0f(&mut self, opcode: u8) {
        let mut full_bytes = Vec::new();

        if self.has_operand_size_prefix {
            full_bytes.push(0x66);
        }
        if self.has_address_size_prefix {
            full_bytes.push(0x67);
        }

        if let Some(oos) = self.opcode_override_segment {
            full_bytes.push(oos);
        }

        full_bytes.push(0x0F);
        full_bytes.push(opcode);
        match opcode {
            0xB7 => {
                if self.has_operand_size_prefix {
                    extended::movzx_r16_rm16(self, &full_bytes);
                } else {
                    extended32::movzx_r32_rm16(self, &full_bytes);
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

    pub fn print_error_exit(&mut self, opcode: u8) {
        let bit_depth = if self.has_operand_size_prefix {
            "opcode32"
        } else {
            "opcode"
        }
        .to_string();

        let bit_address = if self.has_operand_size_prefix {
            "address32"
        } else {
            "address"
        }
        .to_string();

        let bit_extended = if self.has_extended_prefix {
            "extended"
        } else {
            ""
        }
        .to_string();
        error!(
            "Unsupported {bit_depth} {bit_address} {bit_extended} {:#02X} at CS:IP = {:#04x}:{:#04x}",
            opcode,
            self.registers.cs(),
            self.registers.ip()
        );
        self.halted = true;
    }

    fn execute(&mut self, opcode: u8) {
        let mut full_bytes = Vec::new();

        if self.has_operand_size_prefix {
            full_bytes.push(0x66);
        }
        if self.has_address_size_prefix {
            full_bytes.push(0x67);
        }
        full_bytes.push(opcode);
        let csip = [self.registers.cs(), self.registers.ip()];
        match opcode {
            0x2B => {
                if self.has_operand_size_prefix {
                    alu32::sub_r32_rm32(self, &full_bytes);
                } else {
                    alu::sub_r16_rm16(self, &full_bytes);
                }
            }
            0xB4 => {
                mov::mov_ah(self, &full_bytes);
            }
            0xB8 => {
                if !self.has_operand_size_prefix {
                    mov::mov_ax(self, &full_bytes);
                } else {
                    mov32::mov_eax_data(self, &full_bytes);
                }
            }
            0xBA => {
                if !self.has_operand_size_prefix {
                    mov::mov_dx(self, &full_bytes);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xBB => {
                if self.has_operand_size_prefix {
                    mov32::mov_ebx_data(self, &full_bytes);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x8B => {
                if self.has_operand_size_prefix {
                    mov32::mov_r32_rm32(self, &full_bytes);
                } else {
                    mov::mov_r16_rm16(self, &full_bytes);
                }
            }
            0x1f => {
                let _ = self.log_instruction(csip, &full_bytes);
                stack::pop_ds(self);
            }

            0x58 => {
                let _ = self.log_instruction(csip, &full_bytes);
                stack::pop_ax(self);
            }

            0x53 => {
                let _ = self.log_instruction(csip, &full_bytes);
                stack::push_bx(self);
            }

            0x0E => {
                let _ = self.log_instruction(csip, &full_bytes);
                stack::push_cs(self);
            }
            0x50 => {
                let _ = self.log_instruction(csip, &full_bytes);
                stack::push_ax(self);
            }
            0x9C => {
                let _ = self.log_instruction(csip, &full_bytes);
                stack::pushf(self);
            }
            0xA3 => {
                if self.has_operand_size_prefix {
                    mov32::mov_address_eax(self, &full_bytes);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0xC1 => {
                if self.has_operand_size_prefix {
                    alu32::shift_group_c1(self, &full_bytes);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x8C => {
                mov::mov_rm16_sreg(self, &full_bytes);
            }

            0xE8 => {
                control::call(self, &full_bytes);
            }
            0xFC => {
                let _ = self.log_instruction(csip, &full_bytes);
                self.registers.set_flags(self.registers.flags() & !0x0400);
            }
            0xC3 => {
                control::retn(self, &full_bytes);
            }
            0x32 => {
                alu::xor(self, &full_bytes);
            }
            0x74 => {
                control::jz(self, &full_bytes);
            }
            0x9D => {
                let _ = self.log_instruction(csip, &full_bytes);
                stack::popf(self);
            }
            0x77 => {
                control::ja(self, &full_bytes);
            }
            0x80 => {
                alu::group_x80(self, &full_bytes);
            }
            0xCD => {
                system::int(self, &full_bytes);
            }
            0x00 => {
                alu::add_rm8_r8(self, &full_bytes);
            }
            0xFA => {
                self.log_instruction(csip, &full_bytes).ok();
                self.registers.set_flags(self.registers.flags() & !0x0200);
            }
            0xFB => {
                self.log_instruction(csip, &full_bytes).ok();
                self.registers.set_flags(self.registers.flags() | 0x0200);
            }

            0xFF => {
                if self.has_operand_size_prefix {
                    control32::call_rm32(self, &full_bytes);
                    self.print_error_exit(opcode);
                } else {
                    control::call_rm16(self, &full_bytes);
                }
            }

            0x09 => {
                if self.has_operand_size_prefix {
                    alu32::or(self, &full_bytes);
                } else {
                    self.print_error_exit(opcode);
                }
            }
            0x3B => {
                if self.has_operand_size_prefix {
                    alu32::cmp_r32_rm32(self, &full_bytes);
                } else {
                    alu::cmp_r16_rm16(self, &full_bytes);
                }
            }
            0x89 => {
                if !self.has_operand_size_prefix {
                    mov::mov_rm16_r16(self, &full_bytes);
                } else {
                    mov32::mov_rm32_r32(self, &full_bytes);
                }
            }
            0x01 => {
                if self.has_operand_size_prefix {
                    alu32::add_rm32_r32(self, &full_bytes);
                } else {
                    alu::add_rm16_r16(self, &full_bytes);
                }
            }
            0x03 => {
                if self.has_operand_size_prefix {
                    alu32::add_r32_rm32(self, &full_bytes);
                } else {
                    alu::add_r16_rm16(self, &full_bytes);
                }
            }
            _ => {
                self.print_error_exit(opcode);
            }
        }
    }
    pub fn run(&mut self) -> Option<u8> {
        while !self.halted {
            let opcode = self.read_u8(self.registers.cs(), self.registers.ip());
            self.registers.step(None);
            match opcode {
                0x67 => {
                    self.has_address_size_prefix = true;
                }
                0x66 => {
                    self.has_operand_size_prefix = true;
                }
                0x0F => {
                    self.has_extended_prefix = true;
                }
                0x26 => {
                    self.override_segment = Some(self.registers.es());
                } // ES:
                0x2E => {
                    self.override_segment = Some(self.registers.cs());
                } // CS:
                0x36 => {
                    self.override_segment = Some(self.registers.ss());
                } // SS:
                0x3E => {
                    self.override_segment = Some(self.registers.ds());
                } // DS:
                _ => {
                    if self.has_extended_prefix {
                        self.execute_0f(opcode);
                    } else {
                        self.execute(opcode);
                    }
                    self.has_address_size_prefix = false;
                    self.has_operand_size_prefix = false;
                    self.has_extended_prefix = false;
                    self.override_segment = None;
                }
            }
        }
        let exit_code = Some(self.registers.al());
        return exit_code;
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
        let addr = ((segment as u32) << 4).wrapping_add(offset as u32) & 0xFFFFF;
        let addr = addr as usize;
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
        let addr = ((segment as u32) << 4).wrapping_add(offset as u32) & 0xFFFFF;
        let addr = addr as usize;
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
