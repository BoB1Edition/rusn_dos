// Ver: 1 // Ver: 1 File: ./libs/dos_core/src/cpu/run.rs

use crate::{DosMachine, cpu::execute_0f::execute_0f, executor::execute, video};
use std::error::Error;

pub(crate) fn run(machine: &mut DosMachine) -> Result<Option<u8>, Box<dyn Error>> {
    let palette = video::load_vga_palette();
    //let debug = DebugLog::new("debug.log");

    let mut tick_counter: u64 = 0;

    while !machine.halted {
        let opcode = machine.read_instr_u8(machine.registers.ip());
        machine.registers.step(None);

        match opcode {
            0x0F => machine.has_extended_prefix = true,
            0x67 => machine.has_address_size_prefix = true,
            0x66 => machine.has_operand_size_prefix = true,
            0x26 => {
                machine.override_segment = Some(machine.registers.es());
                machine.opcode_override_segment = Some(opcode);
            } // ES:
            0x2E => {
                machine.override_segment = Some(machine.registers.cs());
                machine.opcode_override_segment = Some(opcode);
            } // CS:
            0x36 => {
                machine.override_segment = Some(machine.registers.ss());
                machine.opcode_override_segment = Some(opcode);
            } // SS:
            0x3E => {
                machine.override_segment = Some(machine.registers.ds());
                machine.opcode_override_segment = Some(opcode);
            } // DS:
            0x64 => {
                machine.override_segment = Some(machine.registers.fs());
                machine.opcode_override_segment = Some(opcode);
            } // FS:
            0x65 => {
                machine.override_segment = Some(machine.registers.gs());
                machine.opcode_override_segment = Some(opcode);
            } // GS:
            0xF0 => {
                machine.has_lock_prefix = true; // REPNE
                //machine.rep_prefix_type = Some(0xF0)
            }
            0xF2 => {
                machine.has_rep_prefix = true; // REPNE
                machine.rep_prefix_type = Some(0xF2)
            }
            0xF3 => {
                machine.has_rep_prefix = true;
                machine.rep_prefix_type = Some(0xF3)
            }

            _ => {
                if machine.has_extended_prefix {
                    execute_0f(machine, opcode);
                } else {
                    execute(machine, opcode);
                }
                machine.has_address_size_prefix = false;
                machine.has_operand_size_prefix = false;
                machine.has_extended_prefix = false;
                machine.has_rep_prefix = false;
                machine.has_lock_prefix = false;
                machine.override_segment = None;
                machine.rep_prefix_type = None;
                machine.opcode_override_segment = None;
            }
        }
        if machine.video.mode == video::VideoMode::Mode13h && machine.video.dirty {
            if let Some(fb) = &machine.video.framebuffer {
                let scaled = video::upscale_framebuffer(&fb.data, &palette);
                if let Some(window) = machine.window() {
                    let (width, height) = window.get_size();
                    (*window).update_with_buffer(&scaled, width, height)?;
                }
                machine.video.dirty = false;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        tick_counter += 1;
        if tick_counter >= 65536 {
            crate::instructions::system::call_interrupt(machine, 0x08);
            tick_counter = 0;
        }
    }
    Ok(Some(machine.registers.al()))
}
