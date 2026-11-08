// Ver: 2 File: crate/dos/src/interrupts/dos/exec.rs
use super::misc::set_carry_flag;
use crate::system::{DosSystem, ExecRequest};

/// INT 21h / AH=4Bh — EXEC: загрузить и выполнить программу
pub fn exec_program(sys: &mut DosSystem) {
    log::trace!("INT 21h / AH=4Bh: {}", sys.process_manager.log_prefix);
    match sys.cpu.registers.al() {
        0x00 => {
            let ds = sys.cpu.registers.ds();
            let dx = sys.cpu.registers.dx();
            let es = sys.cpu.registers.es();
            let bx = sys.cpu.registers.bx();
            let filename = sys.filesystem.extract_filename(ds, dx, &sys.machine);
            // Параметрический блок ES:BX
            let env_seg = sys.read_u16(es, bx); // +00 word
            let cmd_off = sys.read_u16(es, bx.wrapping_add(2)); // +02
            let cmd_seg = sys.read_u16(es, bx.wrapping_add(4)); // +04
            let f1_off = sys.read_u16(es, bx.wrapping_add(6)); // +06
            let f1_seg = sys.read_u16(es, bx.wrapping_add(8)); // +08
            let f2_off = sys.read_u16(es, bx.wrapping_add(0x0A)); // +0A
            let f2_seg = sys.read_u16(es, bx.wrapping_add(0x0C)); // +0C
            log::info!(
                "INT 21h / AH=4Bh/AL=00: EXEC '{}' env={:04X} cmd={:04X}:{:04X}",
                filename,
                env_seg,
                cmd_seg,
                cmd_off
            );
            match sys.filesystem.resolve_path(&filename) {
                Ok(p) if p.is_file() => {
                    let host_path = p.to_string_lossy().into_owned();
                    log::info!(
                        "INT 21h / AH=4Bh/AL=00: EXEC '{}' resolved to host path '{}'",
                        filename,
                        host_path
                    );
                    sys.exec_request = Some(ExecRequest {
                        path: host_path,
                        env_seg,
                        cmdline_seg: cmd_seg,
                        cmdline_off: cmd_off,
                        fcb1_seg: f1_seg,
                        fcb1_off: f1_off,
                        fcb2_seg: f2_seg,
                        fcb2_off: f2_off,
                    });
                }
                _ => {
                    log::error!("EXEC: file not found '{}'", filename);
                    set_carry_flag(sys, true);
                    sys.cpu.registers.set_ax(2);
                }
            }
        }
        0x01 => {
            log::warn!("INT 21h / AH=4Bh AL=01 (load, no exec) not supported yet");
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x01);
        }
        0x03 => {
            log::warn!("INT 21h / AH=4Bh AL=03 (overlay) not supported yet");
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x01);
        }
        _ => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x01);
        }
    }
}

/// INT 21h / AH=4Dh — код возврата последнего дочернего процесса
pub fn get_return_code(sys: &mut DosSystem) {
    log::info!(
        "INT 21h / AH=4Dh: child exit code={}, type={}",
        sys.child_exit_code,
        sys.child_exit_type
    );
    sys.cpu
        .registers
        .set_ax(((sys.child_exit_type as u16) << 8) | sys.child_exit_code as u16);
    set_carry_flag(sys, false);
}
