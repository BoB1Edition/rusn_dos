// Ver: 3 File: crate/x86/src/cpu.rs
use crate::tracer::{FileTracer, Tracer};
use crate::{registers::Registers, tracer::NullTracer};
use bus::{Cpu, Machine};

#[derive(Debug, Default)]
pub struct PrefixState {
    pub has_address_size: bool, // Префикс 0x67
    pub has_operand_size: bool, // Префикс 0x66
    pub has_extended: bool,     // Префикс 0x0F
    pub has_rep: bool,
    pub rep_type: Option<u8>,
    pub has_lock: bool,
    pub segment_override: Option<u16>,
}

impl PrefixState {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

pub struct X86Cpu {
    pub registers: Registers,
    pub prefixes: PrefixState,
    pub halted: bool,
    /// Подавление прерываний на одну инструкцию (после MOV SS / POP SS)
    pub inhibit_interrupts: bool,
    /// Вектор программного прерывания, которое шина обязалась обработать
    /// внутренне (DOS/BIOS). Обрабатывается драйвером после возврата из `step`.
    pub pending_interrupt: Option<u8>,
    pub gdtr_limit: u16,
    pub gdtr_base: u32,
    pub idtr_limit: u16,
    pub idtr_base: u32,
    pub cr0: u32,
    pub tracer: Box<dyn Tracer>,
    pub(crate) instruction_bytes: Vec<u8>,
    pub(crate) instruction_start_ip: u16,
}

impl Default for X86Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl X86Cpu {
    /// Создает CPU
    pub fn new() -> Self {
        Self {
            registers: Registers::default(),
            prefixes: PrefixState::default(),
            halted: false,
            inhibit_interrupts: false,
            pending_interrupt: None,
            gdtr_limit: 0,
            gdtr_base: 0,
            idtr_limit: 0,
            idtr_base: 0,
            cr0: 0,
            tracer: Box::new(NullTracer),
            instruction_bytes: Vec::with_capacity(16),
            instruction_start_ip: 0,
        }
    }

    /// Создает CPU и включает запись лога в указанный файл
    pub fn new_with_tracer(log_path: &str) -> std::io::Result<Self> {
        let mut cpu = Self::new();
        cpu.tracer = Box::new(FileTracer::new(log_path)?);
        Ok(cpu)
    }

    #[inline(always)]
    pub(crate) fn phys_ip(&self) -> u32 {
        ((self.registers.cs() as u32) << 4) + self.registers.ip() as u32
    }

    // === Доступ для внешнего диспетчера (мост гибридного режима) ===

    /// Начинает сбор байтов инструкции с указанным стартовым IP
    /// (используется, когда опкод читает внешний цикл, а не `step`).
    pub fn begin_trace(&mut self, start_ip: u16) {
        self.instruction_bytes.clear();
        self.instruction_start_ip = start_ip;
    }

    /// Дописывает байт (префикс/опкод) в буфер трассировки.
    pub fn push_trace_byte(&mut self, b: u8) {
        self.instruction_bytes.push(b);
    }

    /// Собранные байты последней инструкции.
    pub fn trace_bytes(&self) -> &[u8] {
        &self.instruction_bytes
    }

    /// IP начала последней инструкции.
    pub fn trace_start_ip(&self) -> u16 {
        self.instruction_start_ip
    }

    #[inline(always)]
    pub(crate) fn phys_addr(&self, segment: u16, offset: u16) -> u32 {
        ((segment as u32) << 4).wrapping_add(offset as u32)
    }

    /// Читает байт, ЗАПИСЫВАЕТ ЕГО В ЛОГ-БУФЕР и сдвигает IP
    #[inline(always)]
    pub(crate) fn fetch_u8(&mut self, machine: &mut dyn Machine) -> u8 {
        let addr = self.phys_ip();
        let val = machine.read_mem_u8(addr);
        self.instruction_bytes.push(val); // <-- Ключевое изменение
        self.registers.step(None);
        val
    }

    #[inline(always)]
    pub(crate) fn fetch_u16(&mut self, machine: &mut dyn Machine) -> u16 {
        let lo = self.fetch_u8(machine) as u16;
        let hi = self.fetch_u8(machine) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    pub(crate) fn fetch_u32(&mut self, machine: &mut dyn Machine) -> u32 {
        let lo = self.fetch_u16(machine) as u32;
        let hi = self.fetch_u16(machine) as u32;
        lo | (hi << 16)
    }
}

impl Cpu for X86Cpu {
    fn step(&mut self, machine: &mut dyn Machine) {
        if self.halted {
            return;
        }
        self.instruction_bytes.clear();
        self.instruction_start_ip = self.registers.ip();

        loop {
            let addr = self.phys_ip();
            let opcode = machine.read_mem_u8(addr);
            self.instruction_bytes.push(opcode);
            self.registers.step(None);

            match opcode {
                0x0F => {
                    let addr = self.phys_ip();
                    let ext = machine.read_mem_u8(addr);
                    self.instruction_bytes.push(ext);
                    self.registers.step(None);
                    crate::executor::execute_0f(self, machine, ext);
                    self.tracer.log_instruction(
                        self.registers.cs(),
                        self.instruction_start_ip,
                        &self.instruction_bytes,
                    );
                    self.prefixes.clear();
                    break;
                }
                0x67 => self.prefixes.has_address_size = true, // Учет 0x67
                0x66 => self.prefixes.has_operand_size = true, // Учет 0x66
                0x26 => self.prefixes.segment_override = Some(self.registers.es()),
                0x2E => self.prefixes.segment_override = Some(self.registers.cs()),
                0x36 => self.prefixes.segment_override = Some(self.registers.ss()),
                0x3E => self.prefixes.segment_override = Some(self.registers.ds()),
                0x64 => self.prefixes.segment_override = Some(self.registers.fs()),
                0x65 => self.prefixes.segment_override = Some(self.registers.gs()),
                0xF0 => {
                    self.prefixes.has_lock = true;
                    self.prefixes.rep_type = Some(0xF0);
                }
                0xF2 => {
                    self.prefixes.has_rep = true;
                    self.prefixes.rep_type = Some(0xF2);
                }
                0xF3 => {
                    self.prefixes.has_rep = true;
                    self.prefixes.rep_type = Some(0xF3);
                }
                _ => {
                    crate::executor::execute(self, machine, opcode);
                    self.tracer.log_instruction(
                        self.registers.cs(),
                        self.instruction_start_ip,
                        &self.instruction_bytes,
                    );

                    self.prefixes.clear();
                    break;
                }
            }
        }
    }

    fn is_halted(&self) -> bool {
        self.halted
    }
    fn halt(&mut self) {
        self.halted = true;
        self.tracer.flush().ok();
    }
}

// Хелперы для чтения/записи регистров по индексу (нужны для ModRm)
impl X86Cpu {
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