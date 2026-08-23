// Ver: 2 File: crate/x86/src/lib.rs
pub mod registers;
pub mod cpu;
pub mod flags;
pub mod modrm;
pub mod instructions;
pub mod executor;
pub mod tracer;

// Хелперы для чтения/записи регистров по индексу (нужны для ModRm)
impl cpu::X86Cpu {
    #[inline]
    pub fn read_reg8(&self, reg: u8) -> u8 {
        match reg {
            0 => self.registers.al(), 1 => self.registers.cl(),
            2 => self.registers.dl(), 3 => self.registers.bl(),
            4 => self.registers.ah(), 5 => self.registers.ch(),
            6 => self.registers.dh(), 7 => self.registers.bh(),
            _ => unreachable!(),
        }
    }
    
    #[inline]
    pub fn read_reg16(&self, reg: u8) -> u16 {
        match reg {
            0 => self.registers.ax(), 1 => self.registers.cx(),
            2 => self.registers.dx(), 3 => self.registers.bx(),
            4 => self.registers.sp(), 5 => self.registers.bp(),
            6 => self.registers.si(), 7 => self.registers.di(),
            _ => unreachable!(),
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
    pub fn write_reg8(&mut self, reg: u8, val: u8) {
        match reg {
            0 => self.registers.set_al(val), 1 => self.registers.set_cl(val),
            2 => self.registers.set_dl(val), 3 => self.registers.set_bl(val),
            4 => self.registers.set_ah(val), 5 => self.registers.set_ch(val),
            6 => self.registers.set_dh(val), 7 => self.registers.set_bh(val),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn write_reg16(&mut self, reg: u8, val: u16) {
        match reg {
            0 => self.registers.set_ax(val), 1 => self.registers.set_cx(val),
            2 => self.registers.set_dx(val), 3 => self.registers.set_bx(val),
            4 => self.registers.set_sp(val), 5 => self.registers.set_bp(val),
            6 => self.registers.set_si(val), 7 => self.registers.set_di(val),
            _ => unreachable!(),
        }
    }
    #[inline]
    pub fn write_reg32(&mut self, reg: u8, val: u32) {
        match reg {
            0 => self.registers.set_eax(val), 1 => self.registers.set_ecx(val),
            2 => self.registers.set_edx(val), 3 => self.registers.set_ebx(val),
            4 => self.registers.set_esp(val), 5 => self.registers.set_ebp(val),
            6 => self.registers.set_esi(val), 7 => self.registers.set_edi(val),
            _ => unreachable!(),
        }
    }
}