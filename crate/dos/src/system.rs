// Ver: 6 File: crate/dos/src/system.rs

use bus::peripherals::video::{VideoMode, letterbox43, scale_into_rect};
use bus::{Cpu, Display, Machine, Motherboard};
use emu_config::{EnvironmentConfig, MachineConfig};
use x86::cpu::X86Cpu;
use x86::flags;

use crate::filesystem::FileSystem;
use crate::ivt;

#[derive(Debug, Clone)]
pub struct SuspendedCall {
    pub vector: u8,
    pub cs: u16,
    pub ip: u16,
    pub flags: u16,
    pub sp: u16,
    pub ss: u16,
}

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

pub struct DosSystem {
    pub machine: Motherboard,
    pub cpu: X86Cpu,
    pub filesystem: FileSystem,
    pub ems: EmsState,
    pub first_mcb_segment: u16,
    tick_counter: u64,
    last_timer_tick: std::time::Instant,
    pub dta_segment: u16,
    pub dta_offset: u16,
    pub suspended: Option<SuspendedCall>,
    pub machine_config: MachineConfig,
    pub console_input: ConsoleInputState,
    pub scale_buf: Vec<u32>,
    pub exit_code: u8,
    pub program_terminated: bool,
    pub environment: EnvironmentConfig,
    pub break_check: bool,
    pub exec_request: Option<ExecRequest>,
    pub parent_stack: Vec<ParentSnapshot>,
    pub child_exit_code: u8,
    pub child_exit_type: u8,
}

impl DosSystem {
    pub fn new(
        machine: Motherboard,
        cpu: X86Cpu,
        config: MachineConfig,
        environment: EnvironmentConfig,
    ) -> Self {
        let mut machine = machine;
        machine.a20_enabled = config.a20_enable;
        Self {
            machine,
            cpu,
            filesystem: FileSystem::new(),
            ems: EmsState::default(),
            first_mcb_segment: 0x60,
            tick_counter: 0,
            last_timer_tick: std::time::Instant::now(),
            dta_segment: 0,
            dta_offset: 0x0080,
            suspended: None,
            machine_config: config,
            console_input: ConsoleInputState::default(),
            scale_buf: Vec::new(),
            exit_code: 0,
            program_terminated: false,
            environment: environment,
            break_check: false,
            exec_request: None,
            parent_stack: Vec::new(),
            child_exit_code: 0,
            child_exit_type: 0,
        }
    }

    #[inline(always)]
    pub fn get_dta_addr(&self) -> u32 {
        ((self.dta_segment as u32) << 4) + self.dta_offset as u32
    }

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
        self.machine
            .write_mem_u8(Self::phys(segment, offset), value);
    }

    #[inline(always)]
    pub fn write_u16(&mut self, segment: u16, offset: u16, value: u16) {
        self.machine
            .write_mem_u16(Self::phys(segment, offset), value);
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

    pub fn dispatch(&mut self, vector: u8) {
        crate::interrupts::dispatch(self, vector);
    }

    pub fn raise_interrupt(&mut self, vector: u8) {
        if ivt::is_internal(&self.machine, vector) {
            self.dispatch(vector);
            return;
        }
        let flags = self.cpu.registers.flags();
        let cs = self.cpu.registers.cs();
        let ip = self.cpu.registers.ip();
        x86::instructions::stack::push16(&mut self.cpu, &mut self.machine, flags);
        x86::instructions::stack::push16(&mut self.cpu, &mut self.machine, cs);
        x86::instructions::stack::push16(&mut self.cpu, &mut self.machine, ip);

        let mut f = flags;
        f &= !(flags::IF | flags::TF);
        self.cpu.registers.set_flags(f);

        let addr = (vector as u32) * 4;
        let ip = self.machine.read_mem_u16(addr);
        let cs = self.machine.read_mem_u16(addr + 2);
        self.cpu.registers.set_cs(cs);
        self.cpu.registers.set_ip(ip);
    }

    pub fn tick_timer(&mut self) {
        self.tick_counter += 1;
        if self.tick_counter < 910 {
            return;
        }
        self.tick_counter = 0;

        let now = std::time::Instant::now();
        let elapsed_ms = now.duration_since(self.last_timer_tick).as_millis();
        if elapsed_ms >= 54 {
            self.last_timer_tick = now;

            if !self.cpu.inhibit_interrupts && flags::test_if(self.cpu.registers.flags()) {
                self.raise_interrupt(0x08);
            }
        }
    }

    pub fn take_frame(&mut self) -> Option<(Vec<u32>, usize, usize)> {
        if !self.machine.video.dirty {
            return None;
        }
        self.machine.video.dirty = false;
        let pal = self.machine.video.palette_rgb8();
        let (buffer, w, h) = match (self.machine.video.vbe_mode, self.machine.video.mode) {
            (Some(vbe), _) => {
                let buf = self.machine.video.render_vbe(vbe, &pal);
                let (w, h) = vbe.dims();
                (buf, w, h)
            }
            (None, VideoMode::Text80x25) => (self.machine.video.render_text(&pal), 640, 400),
            (None, VideoMode::Mode13h) => (self.machine.video.render_mode13(&pal), 320, 200),
            (None, m) => {
                let (w, h) = m.planar_dims().unwrap_or((320, 200));
                let buf = self.machine.video.render_planar(w, h, &pal);
                (buf, w, h)
            }
        };
        Some((buffer, w, h))
    }

    pub fn render(&mut self, display: &mut dyn Display) {
        let frame = self.take_frame();
        if let Some((buffer, w, h)) = frame {
            let (dw, dh) = display.size();
            if self.scale_buf.len() != dw * dh {
                self.scale_buf.resize(dw * dh, 0);
            }
            let (ox, oy, rw, rh) = letterbox43(dw, dh);
            scale_into_rect(&buffer, w, h, &mut self.scale_buf, dw, dh, ox, oy, rw, rh);
            display.update(&self.scale_buf, dw, dh);
        }
    }

    pub fn step_frame(&mut self, mut display: Option<&mut dyn Display>) {
        self.machine.video.blink_tick();
        const INSTR_BUDGET: usize = 50_000;
        const POLL_INTERVAL: usize = 256;
        let mut executed = 0usize;

        while executed < INSTR_BUDGET && !self.cpu.halted {
            if self.suspended.is_some() {
                if let Some(d) = display.as_deref_mut() {
                    d.pump_events();
                    if !d.is_open() {
                        self.cpu.halted = true;
                        return;
                    }
                    for (scancode, ascii) in d.poll_keys() {
                        self.machine.keyboard.push_key(scancode, ascii);
                    }
                    // Во время ожидания ввода рендерим эхо сразу — это редкие кадры
                    if self.machine.video.dirty {
                        self.render(d);
                    }
                }
                if self.machine.keyboard.has_key() {
                    let vector = self.suspended.take().unwrap().vector;
                    log::debug!("Key available, re-dispatching INT {:02X}h", vector);
                    self.dispatch(vector);
                }
                self.tick_timer();
                executed += 1;
                continue;
            }

            self.step();
            executed += 1;
            if self.exec_request.is_some() {
                match self.start_child() {
                    Ok(()) => {}
                    Err(code) => {
                        log::error!("EXEC failed, error code {}", code);
                        crate::interrupts::dos::misc::set_carry_flag(self, true);
                        self.cpu.registers.set_ax(code as u16);
                    }
                }
                continue;
            }
            // Ребёнок завершился -> восстанавливаем родителя
            if !self.parent_stack.is_empty() && (self.cpu.halted || self.program_terminated) {
                self.finish_child();
                continue;
            }
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
        }
        if self.machine.video.dirty {
            if let Some(d) = display.as_deref_mut() {
                self.render(d);
            } else {
                self.machine.video.dirty = false;
            }
        }
    }

    pub fn is_halted(&self) -> bool {
        self.cpu.halted
    }

    pub fn suspend_for_key(&mut self, vector: u8) {
        self.suspended = Some(SuspendedCall {
            vector,
            cs: 0,
            ip: 0,
            flags: 0,
            sp: 0,
            ss: 0, // больше не используются
        });
        log::debug!("Suspended INT {:02X}h for key wait", vector);
    }
    pub fn start_child(&mut self) -> Result<(), u8> {
        let req = self.exec_request.take().ok_or(0x01u8)?;
        // 1. Копируем command tail и FCB из памяти родителя ДО подмены
        let tail_len = self.read_u8(req.cmdline_seg, req.cmdline_off) as usize;
        let mut tail = Vec::with_capacity(tail_len);
        for i in 0..tail_len as u16 {
            tail.push(self.read_u8(req.cmdline_seg, req.cmdline_off.wrapping_add(1 + i)));
        }
        let mut fcb1 = [0u8; 16];
        let mut fcb2 = [0u8; 16];
        for i in 0..16u16 {
            fcb1[i as usize] = self.read_u8(req.fcb1_seg, req.fcb1_off.wrapping_add(i));
            fcb2[i as usize] = self.read_u8(req.fcb2_seg, req.fcb2_off.wrapping_add(i));
        }
        // 2. Загружаем ребёнка во временный DosSystem
        let mut child = crate::loader::load_executable(
            std::path::PathBuf::from(&req.path),
            true,
            &self.machine_config,
            &self.environment,
            Vec::new(),
        )
        .map_err(|_| 0x0Bu8)?;
        // 3. Снимок родителя в стек (поддержка вложенных EXEC)
        self.parent_stack.push(ParentSnapshot {
            memory: self.machine.memory.as_slice().to_vec(),
            cpu: std::mem::replace(&mut self.cpu, child.cpu),
            dta_segment: self.dta_segment,
            dta_offset: self.dta_offset,
            first_mcb_segment: self.first_mcb_segment,
            console_input: self.console_input.clone(),
            break_check: self.break_check,
        });
        // 4. Пересаживаем память ребёнка (периферия остаётся общей)
        self.machine.memory = std::mem::replace(&mut child.machine.memory, bus::Memory::new(0));
        self.first_mcb_segment = child.first_mcb_segment;
        self.dta_segment = child.dta_segment;
        self.dta_offset = child.dta_offset;
        // 5. Вставляем tail и FCB родителя в PSP ребёнка
        let psp = self.cpu.registers.ds();
        self.write_u8(psp, 0x80, tail_len as u8);
        for (i, &b) in tail.iter().enumerate() {
            self.write_u8(psp, 0x81u16.wrapping_add(i as u16), b);
        }
        self.write_u8(psp, 0x81u16.wrapping_add(tail_len as u16), 0x0D);
        for i in 0..16u16 {
            self.write_u8(psp, 0x5C + i, fcb1[i as usize]);
            self.write_u8(psp, 0x6C + i, fcb2[i as usize]);
        }
        // 6. Сброс рантайм-состояния под ребёнка
        self.program_terminated = false;
        self.exit_code = 0;
        self.cpu.halted = false;
        self.suspended = None;
        self.console_input = ConsoleInputState::default();
        log::info!("EXEC: child '{}' started (tail len={})", req.path, tail_len);
        Ok(())
    }

    pub fn finish_child(&mut self) {
        if let Some(mut snap) = self.parent_stack.pop() {
            self.child_exit_code = self.exit_code;
            self.child_exit_type = if self.program_terminated { 0 } else { 1 };
            let bytes = std::mem::take(&mut snap.memory);
            self.machine.memory = bus::Memory::from_slice(bytes.into_boxed_slice());
            self.cpu = snap.cpu;
            self.cpu.halted = false;
            self.cpu.pending_interrupt = None;
            self.cpu.prefixes.clear();
            self.cpu.inhibit_interrupts = false;
            self.dta_segment = snap.dta_segment;
            self.dta_offset = snap.dta_offset;
            self.first_mcb_segment = snap.first_mcb_segment;
            self.console_input = snap.console_input;
            self.break_check = snap.break_check;
            self.program_terminated = false;
            self.exit_code = 0;
            self.suspended = None;
            // Возврат родителю: CF=0, AX разрушен
            let mut f = self.cpu.registers.flags();
            f &= !x86::flags::CF;
            self.cpu.registers.set_flags(f);
            self.cpu.registers.set_ax(0);
            log::info!(
                "EXEC: child finished (code={}), parent resumed",
                self.child_exit_code
            );
        }
    }
}

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

#[derive(Debug, Clone, Default)]
pub struct ConsoleInputState {
    pub active: bool,
    pub buffer_seg: u16,
    pub buffer_off: u16,
    pub max_chars: u8,
    pub current_count: u8,
}

#[derive(Debug, Clone)]
pub struct ExecRequest {
    pub path: String,
    pub env_seg: u16,
    pub cmdline_seg: u16,
    pub cmdline_off: u16,
    pub fcb1_seg: u16,
    pub fcb1_off: u16,
    pub fcb2_seg: u16,
    pub fcb2_off: u16,
}

pub struct ParentSnapshot {
    memory: Vec<u8>,
    cpu: X86Cpu,
    dta_segment: u16,
    dta_offset: u16,
    first_mcb_segment: u16,
    console_input: ConsoleInputState,
    break_check: bool,
}
