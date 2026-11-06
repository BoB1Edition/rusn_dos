// Ver: 3 File: crate/dos/src/process.rs
use x86::tracer::Tracer;
use crate::system::ConsoleInputState;

pub struct ProcessManager {
    pub parent_stack: Vec<ParentSnapshot>,
    pub log_prefix: String,
    pub trace_enabled: bool,
}

pub struct ParentSnapshot {
    pub registers: x86::registers::Registers,
    pub tracer: Box<dyn Tracer>,
    pub dta_segment: u16,
    pub dta_offset: u16,
    pub first_mcb_segment: u16,
    pub console_input: ConsoleInputState,
    pub break_check: bool,
    pub child_psp_segment: u16,
    pub log_prefix: String,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self {
            parent_stack: Vec::new(),
            log_prefix: String::new(),
            trace_enabled: true,
        }
    }
}

impl ProcessManager {
    pub fn start_child(
        &mut self,
        sys: &mut crate::system::DosSystem,
    ) -> Result<(), u8> {
        let req = sys.exec_request.take().ok_or(0x01u8)?;

        let child_name = std::path::Path::new(&req.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let new_log_prefix = if self.log_prefix.is_empty() {
            child_name.clone()
        } else {
            let prefix = format!("{}_{}", self.log_prefix, child_name);
            prefix
        };
        let parent_registers = sys.cpu.registers.clone();
        let parent_dta_segment = sys.dta_segment;
        let parent_dta_offset = sys.dta_offset;
        let parent_first_mcb_segment = sys.first_mcb_segment;
        let parent_console_input = sys.console_input.clone();
        let parent_break_check = sys.break_check;
        let parent_log_prefix = self.log_prefix.clone();
        let tail_len = sys.read_u8(req.cmdline_seg, req.cmdline_off) as usize;
        let mut tail = Vec::with_capacity(tail_len);
        for i in 0..tail_len as u16 {
            tail.push(sys.read_u8(
                req.cmdline_seg,
                req.cmdline_off.wrapping_add(1 + i),
            ));
        }

        let mut fcb1 = [0u8; 16];
        let mut fcb2 = [0u8; 16];
        for i in 0..16u16 {
            fcb1[i as usize] = sys.read_u8(req.fcb1_seg, req.fcb1_off.wrapping_add(i));
            fcb2[i as usize] = sys.read_u8(req.fcb2_seg, req.fcb2_off.wrapping_add(i));
        }
        let psp_seg =
            match crate::loader::detect_executable_type(&std::path::PathBuf::from(&req.path)) {
                crate::loader::ExecutableType::Exe => {
                    crate::loader::exe_loader::load_in_place(sys, &req, &tail, &fcb1, &fcb2)?
                }
                crate::loader::ExecutableType::Com => {
                    crate::loader::com_loader::load_in_place(sys, &req, &tail, &fcb1, &fcb2)?
                }
            };
        let parent_tracer =
            std::mem::replace(&mut sys.cpu.tracer, Box::new(x86::tracer::NullTracer));

        self.parent_stack.push(ParentSnapshot {
            registers: parent_registers,
            tracer: parent_tracer,
            dta_segment: parent_dta_segment,
            dta_offset: parent_dta_offset,
            first_mcb_segment: parent_first_mcb_segment,
            console_input: parent_console_input,
            break_check: parent_break_check,
            child_psp_segment: psp_seg,
            log_prefix: parent_log_prefix,
        });
        if self.trace_enabled {
            let log_filename = format!("logopcode_{}.txt", new_log_prefix);
            sys.cpu.tracer = match x86::tracer::FileTracer::new(&log_filename) {
                Ok(t) => {
                    log::info!("Child tracer created: {}", log_filename);
                    Box::new(t)
                }
                Err(e) => {
                    log::warn!(
                        "Failed to create child tracer {}: {}. Using NullTracer.",
                        log_filename,
                        e
                    );
                    Box::new(x86::tracer::NullTracer)
                }
            };
        } else {
            sys.cpu.tracer = Box::new(x86::tracer::NullTracer);
        }

        self.log_prefix = new_log_prefix;
        sys.program_terminated = false;
        sys.exit_code = 0;
        sys.cpu.halted = false;
        sys.cpu.pending_interrupt = None;
        sys.cpu.prefixes.clear();
        sys.cpu.inhibit_interrupts = false;
        sys.suspended = None;
        sys.console_input = ConsoleInputState::default();

        log::info!(
            "EXEC: child '{}' started in-place at PSP {:04X}",
            req.path,
            psp_seg
        );

        Ok(())
    }

    pub fn finish_child(&mut self, sys: &mut crate::system::DosSystem) {
        if let Some(snap) = self.parent_stack.pop() {
            sys.child_exit_code = sys.exit_code;
            sys.child_exit_type = if sys.program_terminated { 0 } else { 1 };
            if snap.child_psp_segment != 0 {
                let _ = crate::mcb::free(
                    &mut sys.machine,
                    snap.first_mcb_segment,
                    snap.child_psp_segment,
                );
            }
            let _ = sys.cpu.tracer.flush();
            sys.cpu.registers = snap.registers;
            sys.cpu.tracer = snap.tracer;
            sys.cpu.halted = false;
            sys.cpu.pending_interrupt = None;
            sys.cpu.prefixes.clear();
            sys.cpu.inhibit_interrupts = false;

            sys.dta_segment = snap.dta_segment;
            sys.dta_offset = snap.dta_offset;
            sys.first_mcb_segment = snap.first_mcb_segment;
            sys.console_input = snap.console_input;
            sys.break_check = snap.break_check;

            sys.program_terminated = false;
            sys.exit_code = 0;
            sys.suspended = None;
            self.log_prefix = snap.log_prefix;
            let mut f = sys.cpu.registers.flags();
            f &= !x86::flags::CF;
            sys.cpu.registers.set_flags(f);
            sys.cpu.registers.set_ax(sys.child_exit_code as u16);

            log::info!(
                "EXEC: child finished (code={}), parent resumed",
                sys.child_exit_code
            );
        }
    }
}