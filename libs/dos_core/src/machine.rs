// Ver: 2
use std::{error::Error, fs::File, io::Write};

use log::error;
use minifb::Window;

use crate::{
    filesystem::FileSystem,
    interrupts,
    memory::Memory,
    registers::Registers,
    video::{VideoMode, VideoSystem},
};

#[derive(Debug)]
pub struct DosMachine {
    pub(crate) memory: Memory,
    pub(crate) registers: crate::registers::Registers,
    pub(crate) halted: bool,
    pub(crate) logfile: File,
    pub(crate) has_address_size_prefix: bool,
    pub(crate) has_operand_size_prefix: bool,
    pub(crate) has_extended_prefix: bool,
    pub(crate) override_segment: Option<u16>,
    pub(crate) opcode_override_segment: Option<u8>,
    pub(crate) video: VideoSystem,
    pub(crate) serial_buffer: Option<Vec<u8>>,
    pub filesystem: FileSystem,
    window: Option<*mut Window>,
    pub(crate) has_rep_prefix: bool,
    pub(crate) rep_prefix_type: Option<u8>,
    pub(crate) keyboard_led_command_pending: bool,
    pub(crate) timer_channel_0: Option<u8>,
    pub(crate) timer_channel_1: Option<u8>,
    pub(crate) timer_channel_2: Option<u8>,
    pub(crate) timer_initialized: bool,
    pub(crate) a20_enabled: bool,
    pub(crate) a20_command_pending: bool,
    pub(crate) keyboard_status: u8,
    pub(crate) ems_page_frame_segment: u16,
    pub(crate) ems_total_pages: u16,
    pub(crate) ems_free_pages: u16,
    pub(crate) ems_next_handle: u16,
    pub(crate) ems_handles: Vec<(u16, u16)>,
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
            4 => self.registers.fs(),
            5 => self.registers.gs(),
            _ => 0, // зарезервировано
        }
    }

    #[inline]
    pub fn read_reg32(&self, reg: u8) -> u32 {
        match reg {
            0 => self.registers.eax(),
            1 => self.registers.ecx(),
            2 => self.registers.edx(),
            3 => self.registers.ebx(),
            4 => self.registers.esp(),
            5 => self.registers.ebp(),
            6 => self.registers.esi(),
            7 => self.registers.edi(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn write_reg32(&mut self, reg: u8, value: u32) {
        match reg {
            0 => self.registers.set_eax(value),
            1 => self.registers.set_ecx(value),
            2 => self.registers.set_edx(value),
            3 => self.registers.set_ebx(value),
            4 => self.registers.set_esp(value),
            5 => self.registers.set_ebp(value),
            6 => self.registers.set_esi(value),
            7 => self.registers.set_edi(value),
            _ => unreachable!(),
        }
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

    pub fn print_error_exit(&mut self, opcode: u8) {
        let bit_depth = if self.has_operand_size_prefix {
            "opcode32"
        } else {
            "opcode"
        }
        .to_string();

        let bit_address = if self.has_address_size_prefix {
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
        self.has_address_size_prefix = false;
        self.has_operand_size_prefix = false;
        self.has_extended_prefix = false;
        self.override_segment = None;
        self.opcode_override_segment = None;
    }

    pub fn run(&mut self, window: Option<&mut Window>) -> Result<Option<u8>, Box<dyn Error>> {
        self.window = window.map(|w| w as *mut Window);
        crate::executor::run(self)
    }

    pub fn handle_int21(&mut self) {
        interrupts::dos::handle_int21(self);
    }

    #[inline(always)]
    pub fn read_u8(&self, default_segment: u16, offset: u16) -> u8 {
        let segment = self.override_segment.unwrap_or(default_segment);
        let addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        let addr = self.apply_a20_mask(addr);
        self.memory.read_u8(addr)
    }

    #[inline(always)]
    pub fn read_u16(&self, default_segment: u16, offset: u16) -> u16 {
        let segment = self.override_segment.unwrap_or(default_segment);
        let lo = self.read_u8(segment, offset) as u16;
        let hi = self.read_u8(segment, offset.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    pub fn read_u32(&self, default_segment: u16, offset: u16) -> u32 {
        let segment = self.override_segment.unwrap_or(default_segment);
        let b0 = self.read_u8(segment, offset) as u32;
        let b1 = self.read_u8(segment, offset.wrapping_add(1)) as u32;
        let b2 = self.read_u8(segment, offset.wrapping_add(2)) as u32;
        let b3 = self.read_u8(segment, offset.wrapping_add(3)) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    #[inline(always)]
    pub(crate) fn write_u8(&mut self, default_segment: u16, offset: u16, value: u8) {
        let segment = self.override_segment.unwrap_or(default_segment);
        let raw_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        let addr = self.apply_a20_mask(raw_addr);
        if addr >= 0xA0000 && addr < 0xC0000 {
            if self.video.mode == VideoMode::Mode13h && addr < 0xA0000 + 320 * 200 {
                if let Some(fb) = self.video.framebuffer.as_mut() {
                    let video_offset = (addr - 0xA0000) as usize;
                    fb.data[video_offset] = value;
                    self.video.dirty = true;
                }
            }
            return;
        }

        if addr < self.memory.len() as u32 {
            self.memory.write_u8(addr, value);
        } else {
            log::error!("Memory write out of bounds: {:#x}", addr);
        }
    }

    #[inline(always)]
    pub fn write_u16(&mut self, default_segment: u16, offset: u16, value: u16) {
        let segment = self.override_segment.unwrap_or(default_segment);
        self.write_u8(segment, offset, value as u8);
        self.write_u8(segment, offset.wrapping_add(1), (value >> 8) as u8);
    }

    #[inline(always)]
    pub fn write_u32(&mut self, default_segment: u16, offset: u16, value: u32) {
        let segment = self.override_segment.unwrap_or(default_segment);
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
    pub fn read_phys_u8(&self, addr: u32) -> u8 {
        let masked = self.apply_a20_mask(addr);
        self.memory.read_u8(masked)
    }

    #[inline(always)]
    pub fn read_phys_u16(&self, addr: u32) -> u16 {
        let lo = self.read_phys_u8(addr) as u16;
        let hi = self.read_phys_u8(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    pub fn read_phys_u32(&self, addr: u32) -> u32 {
        let lo = self.read_phys_u16(addr) as u32;
        let hi = self.read_phys_u16(addr.wrapping_add(2)) as u32;
        lo | (hi << 16)
    }

    #[inline(always)]
    pub fn write_phys_u8(&mut self, addr: u32, value: u8) {
        let masked = self.apply_a20_mask(addr);
        self.memory.write_u8(masked, value);
    }

    #[inline(always)]
    pub fn write_phys_u16(&mut self, addr: u32, value: u16) {
        self.write_phys_u8(addr, value as u8);
        self.write_phys_u8(addr.wrapping_add(1), (value >> 8) as u8);
    }

    #[inline(always)]
    pub fn write_phys_u32(&mut self, addr: u32, value: u32) {
        self.write_phys_u16(addr, value as u16);
        self.write_phys_u16(addr.wrapping_add(2), (value >> 16) as u16);
    }

    pub fn print_char(&self) {
        let ch = self.registers.dl() as char;
        print!("{}", ch);
        std::io::stdout().flush().ok();
    }

    pub fn handle_int2f(&mut self) {
        interrupts::dos::handle_int2f(self);
    }

    pub fn new_with_memory(memory: Memory, logfile: File) -> Self {
        Self {
            memory,
            registers: Registers::default(),
            halted: false,
            logfile,
            has_address_size_prefix: false,
            has_operand_size_prefix: false,
            has_extended_prefix: false,
            override_segment: None,
            opcode_override_segment: None,
            video: VideoSystem::new(),
            window: None,
            has_rep_prefix: false,
            filesystem: FileSystem::new(),
            rep_prefix_type: None,
            keyboard_led_command_pending: false,
            timer_initialized: false,
            timer_channel_0: None,
            timer_channel_1: None,
            timer_channel_2: None,
            serial_buffer: Some(Vec::new()),
            a20_enabled: false,
            a20_command_pending: false,
            keyboard_status: 0x18,
            ems_page_frame_segment: 0xD000, // Стандартный фрейм EMS
            ems_total_pages: 256,           // 4 MB памяти (256 * 16KB)
            ems_free_pages: 256,
            ems_next_handle: 1,
            ems_handles: Vec::new(),
        }
    }

    pub(crate) fn window(&mut self) -> Option<&mut Window> {
        self.window.map(|ptr| unsafe { &mut *ptr })
    }

    #[inline(always)]
    pub(crate) fn apply_a20_mask(&self, addr: u32) -> u32 {
        if addr >= 0x0FFFF0 {
            log::debug!(
                "A20 MASK: raw={:#x}, enabled={}, result={:#x}",
                addr,
                self.a20_enabled,
                if self.a20_enabled {
                    addr.min(0x10FFFF)
                } else {
                    addr & 0xFFFFF
                }
            );
        }

        if self.a20_enabled {
            addr.min(0x10FFFF)
        } else {
            addr & 0xFFFFF
        }
    }
    #[inline(always)]
    pub fn read_instr_u8(&self, offset: u16) -> u8 {
        let addr = ((self.registers.cs() as u32) << 4).wrapping_add(offset as u32);
        let addr = self.apply_a20_mask(addr);
        self.memory.read_u8(addr)
    }
}

impl Drop for DosMachine {
    fn drop(&mut self) {}
}
