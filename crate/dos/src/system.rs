// Ver: 1 File: crate/dos/src/system.rs

use bus::peripherals::video::{
    get_fonts_vga8x16, load_vga_palette, render_text_to_pixels, scale_buffer,
    upscale_framebuffer, VideoMode,
};
use bus::{Cpu, Display, Machine, Motherboard};
use x86::cpu::X86Cpu;
use x86::flags;

use crate::filesystem::FileSystem;
use crate::ivt;

/// Состояние EMS (LIM 4.0).
#[derive(Debug, Clone)]
pub struct EmsState {
    pub page_frame_segment: u16,
    pub total_pages: u16,
    pub free_pages: u16,
    pub next_handle: u16,
    pub handles: Vec<(u16, u16)>,
}

impl Default for EmsState {
    fn default() -> Self {
        Self {
            page_frame_segment: 0xD000,
            total_pages: 256,
            free_pages: 256,
            next_handle: 1,
            handles: Vec::new(),
        }
    }
}

/// Главный объект DOS-эмулятора: шина (Motherboard) + CPU (X86Cpu) + DOS-слой.
pub struct DosSystem {
    pub machine: Motherboard,
    pub cpu: X86Cpu,
    pub filesystem: FileSystem,
    pub ems: EmsState,
    pub first_mcb_segment: u16,
    tick_counter: u64,
}

impl DosSystem {
    pub fn new(machine: Motherboard, cpu: X86Cpu) -> Self {
        Self {
            machine,
            cpu,
            filesystem: FileSystem::new(),
            ems: EmsState::default(),
            first_mcb_segment: 0x60,
            tick_counter: 0,
        }
    }

    // === Адресация памяти ===

    #[inline(always)]
    pub fn phys(segment: u16, offset: u16) -> u32 {
        ((segment as u32) << 4).wrapping_add(offset as u32)
    }

    #[inline(always)]
    pub fn read_u8(&self, segment: u16, offset: u16) -> u8 {
        self.machine.read_mem_u8(Self::phys(segment, offset))
    }

    #[inline(always)]
    pub fn read_u16(&self, segment: u16, offset: u16) -> u16 {
        self.machine.read_mem_u16(Self::phys(segment, offset))
    }

    #[inline(always)]
    pub fn write_u8(&mut self, segment: u16, offset: u16, value: u8) {
        self.machine.write_mem_u8(Self::phys(segment, offset), value);
    }

    #[inline(always)]
    pub fn write_u16(&mut self, segment: u16, offset: u16, value: u16) {
        self.machine.write_mem_u16(Self::phys(segment, offset), value);
    }

    #[inline(always)]
    pub fn read_phys_u8(&self, addr: u32) -> u8 {
        self.machine.read_mem_u8(addr)
    }

    #[inline(always)]
    pub fn read_phys_u16(&self, addr: u32) -> u16 {
        self.machine.read_mem_u16(addr)
    }

    #[inline(always)]
    pub fn read_phys_u32(&self, addr: u32) -> u32 {
        self.machine.read_mem_u32(addr)
    }

    #[inline(always)]
    pub fn write_phys_u8(&mut self, addr: u32, value: u8) {
        self.machine.write_mem_u8(addr, value);
    }

    #[inline(always)]
    pub fn write_phys_u16(&mut self, addr: u32, value: u16) {
        self.machine.write_mem_u16(addr, value);
    }

    #[inline(always)]
    pub fn write_phys_u32(&mut self, addr: u32, value: u32) {
        self.machine.write_mem_u32(addr, value);
    }

    // === Исполнение ===

    /// Выполняет одну инструкцию CPU и, при необходимости, внутреннее прерывание.
    pub fn step(&mut self) {
        {
            let mut bus = CpuBus {
                mem: &mut self.machine,
            };
            self.cpu.step(&mut bus);
        }
        if let Some(vector) = self.cpu.pending_interrupt.take() {
            self.dispatch(vector);
        }
    }

    /// Передаёт управление внутреннему обработчику прерывания (CS=0xF000).
    pub fn dispatch(&mut self, vector: u8) {
        crate::interrupts::dispatch(self, vector);
    }

    /// Поднимает прерывание: внутреннее обрабатывается сразу, внешнее — через
    /// настоящий дальний вызов по адресу из IVT.
    pub fn raise_interrupt(&mut self, vector: u8) {
        if ivt::is_internal(&self.machine, vector) {
            self.dispatch(vector);
            return;
        }
        let ss_base = (self.cpu.registers.ss() as u32) << 4;
        let mut sp = self.cpu.registers.sp();
        sp = sp.wrapping_sub(2);
        self.cpu.registers.set_sp(sp);
        self.machine
            .write_mem_u16(ss_base.wrapping_add(sp as u32), self.cpu.registers.flags());
        sp = sp.wrapping_sub(2);
        self.cpu.registers.set_sp(sp);
        self.machine
            .write_mem_u16(ss_base.wrapping_add(sp as u32), self.cpu.registers.cs());
        sp = sp.wrapping_sub(2);
        self.cpu.registers.set_sp(sp);
        self.machine
            .write_mem_u16(ss_base.wrapping_add(sp as u32), self.cpu.registers.ip());

        let mut f = self.cpu.registers.flags();
        f &= !(flags::IF | flags::TF);
        self.cpu.registers.set_flags(f);

        let addr = (vector as u32) * 4;
        let ip = self.machine.read_mem_u16(addr);
        let cs = self.machine.read_mem_u16(addr + 2);
        self.cpu.registers.set_cs(cs);
        self.cpu.registers.set_ip(ip);
    }

    /// Псевдо-таймер: каждые 65536 выполненных инструкций поднимает IRQ0 (INT 08h).
    pub fn tick_timer(&mut self) {
        self.tick_counter += 1;
        if self.tick_counter >= 65536 {
            if !self.cpu.inhibit_interrupts && flags::test_if(self.cpu.registers.flags()) {
                self.raise_interrupt(0x08);
            }
            self.tick_counter = 0;
        }
        if self.cpu.inhibit_interrupts {
            self.cpu.inhibit_interrupts = false;
        }
    }

    // === Рендеринг ===

    /// Формирует кадр из видеопамяти, если она помечена как «грязная».
    /// Возвращает (пиксели, ширина, высота) и сбрасывает флаг dirty.
    pub fn take_frame(&mut self) -> Option<(Vec<u32>, usize, usize)> {
        if !self.machine.video.dirty {
            return None;
        }
        self.machine.video.dirty = false;
        let (buffer, w, h) = match self.machine.video.mode {
            VideoMode::Text80x25 => {
                let buf = render_text_to_pixels(
                    &self.machine.video.text_buffer,
                    &get_fonts_vga8x16(),
                );
                (buf, 640, 400)
            }
            VideoMode::Mode13h => {
                let buf =
                    upscale_framebuffer(&self.machine.video.frame_buffer, &load_vga_palette());
                (buf, 1920, 1200)
            }
        };
        Some((buffer, w, h))
    }

    /// Отображает кадр через `Display`, при необходимости масштабируя его.
    pub fn render(&mut self, display: &mut dyn Display) {
        let frame = self.take_frame();
        if let Some((buffer, w, h)) = frame {
            let (dw, dh) = display.size();
            let scaled = if w != dw || h != dh {
                scale_buffer(&buffer, w, h, dw, dh)
            } else {
                buffer
            };
            display.update(&scaled, dw, dh);
        }
    }

    /// Один шаг эмуляции верхнего уровня: пакет инструкций, ввод, рендеринг, таймер.
    pub fn step_frame(&mut self, mut display: Option<&mut dyn Display>) {
        const INSTR_BUDGET: usize = 50_000;
        const POLL_INTERVAL: usize = 256;

        let mut executed = 0usize;
        while executed < INSTR_BUDGET && !self.cpu.halted {
            self.step();
            executed += 1;
            self.tick_timer();

            if executed % POLL_INTERVAL == 0 {
                if let Some(d) = display.as_deref_mut() {
                    if !d.is_open() {
                        self.cpu.halted = true;
                        return;
                    }
                    for (scancode, ascii) in d.poll_keys() {
                        self.machine.keyboard.push_key(scancode, ascii);
                    }
                }
            }

            if self.machine.video.dirty {
                if let Some(d) = display.as_deref_mut() {
                    self.render(d);
                } else {
                    self.machine.video.dirty = false;
                }
                // Кадр отрисован — уступаем управление внешнему циклу для темпа.
                return;
            }
        }
    }

    pub fn is_halted(&self) -> bool {
        self.cpu.halted
    }
}

/// Мостик `Machine` для CPU: делегирует шине и знает, какие векторы
/// обслуживает DOS-слой (CS=0xF000 → внутренние).
struct CpuBus<'a> {
    mem: &'a mut Motherboard,
}

impl Machine for CpuBus<'_> {
    fn read_mem_u8(&self, addr: u32) -> u8 {
        self.mem.read_mem_u8(addr)
    }
    fn write_mem_u8(&mut self, addr: u32, val: u8) {
        self.mem.write_mem_u8(addr, val);
    }
    fn in_port(&mut self, port: u16) -> u8 {
        self.mem.in_port(port)
    }
    fn out_port(&mut self, port: u16, val: u8) {
        self.mem.out_port(port, val);
    }
    fn trigger_irq(&mut self, irq_line: u8) {
        self.mem.trigger_irq(irq_line);
    }
    fn handle_software_interrupt(&mut self, vector: u8) -> bool {
        ivt::is_internal(self.mem, vector)
    }
    fn is_halted(&self) -> bool {
        self.mem.is_halted()
    }
    fn halt(&mut self) {
        self.mem.halt();
    }
}

/// Реализация `Machine` для `DosSystem`: делегирование шине + DOS-семантика прерываний.
impl Machine for DosSystem {
    fn read_mem_u8(&self, addr: u32) -> u8 {
        self.machine.read_mem_u8(addr)
    }
    fn write_mem_u8(&mut self, addr: u32, val: u8) {
        self.machine.write_mem_u8(addr, val);
    }
    fn in_port(&mut self, port: u16) -> u8 {
        self.machine.in_port(port)
    }
    fn out_port(&mut self, port: u16, val: u8) {
        self.machine.out_port(port, val);
    }
    fn trigger_irq(&mut self, irq_line: u8) {
        self.machine.trigger_irq(irq_line);
    }
    fn handle_software_interrupt(&mut self, vector: u8) -> bool {
        ivt::is_internal(&self.machine, vector)
    }
    fn is_halted(&self) -> bool {
        self.cpu.halted
    }
    fn halt(&mut self) {
        self.cpu.halted = true;
    }
}
