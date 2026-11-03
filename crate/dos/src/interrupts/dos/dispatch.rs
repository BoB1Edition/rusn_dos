// Ver: 7 File: crate/dos/src/interrupts/dos/dispatch.rs
use x86::flags;

use crate::interrupts::dos::exec;
use crate::interrupts::dos::file_ops::file_date_time;
use crate::interrupts::dos::misc::{break_check_handler, get_country_info, get_system_date, get_system_time};
use crate::system::DosSystem;

use super::console::{print_char, direct_console_io, direct_char_input_no_echo, char_input_no_echo, print_dos_string, buffered_input, read_char_with_echo};
use super::misc::{set_carry_flag, set_dta, get_interrupt_vector, set_interrupt_vector, ioctl};
use super::file_ops::{open_file, close_file, read_file, write_file, seek_file, delete_file, create_file, file_attributes};
use super::dir::{set_current_directory, get_current_directory, get_current_drive};
use super::memory::{allocate_memory_handler, free_memory_handler, modify_memory_handler};
use super::find::{find_first, find_next};

pub fn handle_int21(sys: &mut DosSystem) {
    log::trace!("int 21: {:#02x}", sys.cpu.registers.ah());
    match sys.cpu.registers.ah() {
        0x00 => {
            log::info!("Program terminated via INT 21h AH=00h, exit code={}", sys.cpu.registers.al());
            sys.exit_code = sys.cpu.registers.al();
            sys.program_terminated = true;
            sys.cpu.halted = true;
        }
        0x01 => read_char_with_echo(sys),
        0x02 => print_char(sys),
        0x06 => direct_console_io(sys),
        0x07 => direct_char_input_no_echo(sys),
        0x08 => char_input_no_echo(sys),
        0x09 => print_dos_string(sys),
        0x0A => buffered_input(sys),
        0x19 => get_current_drive(sys),
        0x1A => set_dta(sys),
        0x25 => set_interrupt_vector(sys),
        0x2A => get_system_date(sys),
        0x2C => get_system_time(sys),
        0x30 => {
            sys.cpu.registers.set_ax(0x1606);
            sys.cpu.registers.set_bx(0x0000);
            sys.cpu.registers.set_cx(0x0000);
            set_carry_flag(sys, false);
        }
        0x33 => break_check_handler(sys),
        0x35 => get_interrupt_vector(sys),
        0x38 => get_country_info(sys),
        0x3B => set_current_directory(sys),
        0x3C => create_file(sys),
        0x3D => open_file(sys),
        0x3E => close_file(sys),
        0x3F => read_file(sys),
        0x40 => write_file(sys),
        0x41 => delete_file(sys),
        0x42 => seek_file(sys),
        0x43 => file_attributes(sys),
        0x44 => ioctl(sys),
        0x47 => get_current_directory(sys),
        0x48 => allocate_memory_handler(sys),
        0x49 => free_memory_handler(sys),
        0x4A => modify_memory_handler(sys),
        0x4B => exec::exec_program(sys),
        0x4C => {
            log::debug!("Program terminated via INT 21h AH=4Ch, exit code={}", sys.cpu.registers.al());
            sys.exit_code = sys.cpu.registers.al();
            sys.program_terminated = true;
            sys.cpu.halted = true
        }
        0x4D => exec::get_return_code(sys),
        0x4E => find_first(sys),
        0x4F => find_next(sys),
        0x57 => file_date_time(sys),
        _ => {
            log::warn!(
                "Unsupported DOS call 21h/{:02X}h at CS:IP={:04X}:{:04X}",
                sys.cpu.registers.ah(),
                sys.cpu.registers.cs(),
                sys.cpu.registers.ip()
            );
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x0001);
            sys.cpu.halted = true
        }
    }
}

pub fn handle_int2f(sys: &mut DosSystem) {
    let ax = sys.cpu.registers.ax();
    let ah = (ax >> 8) as u8;
    match ax {
        0x1687 => {
            if sys.machine_config.dpmi {
                let mut f = sys.cpu.registers.flags();
                f &= !flags::CF;
                sys.cpu.registers.set_flags(f);
                sys.cpu.registers.set_ax(0x0001);
                log::info!("INT 2Fh / AX=1687h: DPMI Host Check -> Installed (config)");
            } else {
                let mut f = sys.cpu.registers.flags();
                f |= flags::CF;
                sys.cpu.registers.set_flags(f);
                log::info!("INT 2Fh / AX=1687h: DPMI Host Check -> Not Installed (CF=1)");
            }
            return;
        }
        0x0C00 => {
            sys.cpu.registers.set_ax(0);
            return;
        }
        _ => {}
    }
    match ah {
        0x16 => {
            let mut f = sys.cpu.registers.flags();
            f |= flags::CF;
            sys.cpu.registers.set_flags(f);
        }
        0x4A => sys.cpu.registers.set_al(0),
        0x12 => sys.cpu.registers.set_al(0),
        0x0C => sys.cpu.registers.set_al(0),
        0x0D => sys.cpu.registers.set_al(0),
        _ => sys.cpu.registers.set_al(0),
    }
}
