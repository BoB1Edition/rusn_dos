use std::io::Write;
use bus::Machine;
use crate::system::DosSystem;
use crate::interrupts::bios::tty_write;
use x86::flags;

use super::misc::set_carry_flag;

pub fn print_char(sys: &mut DosSystem) {
    tty_write(sys, sys.cpu.registers.dl());
}

pub fn direct_console_io(sys: &mut DosSystem) {
    let dl = sys.cpu.registers.dl();

    if dl == 0xFF {
        if let Some(key) = sys.machine.keyboard.pop_key() {
            let ascii = (key & 0xFF) as u8;
            sys.cpu.registers.set_al(ascii);
            let mut f = sys.cpu.registers.flags();
            f &= !flags::ZF;
            sys.cpu.registers.set_flags(f);
        } else {
            sys.cpu.registers.set_al(0);
            let mut f = sys.cpu.registers.flags();
            f |= flags::ZF;
            sys.cpu.registers.set_flags(f);
        }
    } else {
        tty_write(sys, dl);
        sys.cpu.registers.set_al(dl);
        let mut f = sys.cpu.registers.flags();
        f &= !flags::ZF;
        sys.cpu.registers.set_flags(f);
    }
}

pub fn print_dos_string(sys: &mut DosSystem) {
    let mut addr = DosSystem::phys(sys.cpu.registers.ds(), sys.cpu.registers.dx());
    loop {
        let byte = sys.machine.read_mem_u8(addr);
        if byte == b'$' {
            break;
        }
        tty_write(sys, byte);
        addr = addr.wrapping_add(1);
    }
}

pub fn read_char_with_echo(sys: &mut DosSystem) {
    if let Some(key) = sys.machine.keyboard.pop_key() {
        let ascii = (key & 0xFF) as u8;
        if ascii >= 0x20 {
            tty_write(sys, ascii);
        }

        sys.cpu.registers.set_al(ascii);
        let mut f = sys.cpu.registers.flags();
        f &= !flags::ZF;
        sys.cpu.registers.set_flags(f);
    } else {
        sys.suspend_for_key(0x21);
    }
}

pub fn direct_char_input_no_echo(sys: &mut DosSystem) {
    if let Some(key) = sys.machine.keyboard.pop_key() {
        let ascii = (key & 0xFF) as u8;
        sys.cpu.registers.set_al(ascii);
    } else {
        sys.suspend_for_key(0x21);
    }
}

pub fn char_input_no_echo(sys: &mut DosSystem) {
    if let Some(key) = sys.machine.keyboard.pop_key() {
        let ascii = (key & 0xFF) as u8;
        if ascii == 0x03 {
            log::info!("INT 21h/AH=08h: Ctrl+C, raising INT 23h");
            sys.raise_interrupt(0x23);
            sys.suspend_for_key(0x21);
            return;
        }
        sys.cpu.registers.set_al(ascii);
    } else {
        sys.suspend_for_key(0x21);
    }
}

pub fn buffered_input(sys: &mut DosSystem) {
    let buffer_seg = sys.cpu.registers.ds();
    let buffer_off = sys.cpu.registers.dx();

    if !sys.console_input.active {
        let max_chars = sys.read_u8(buffer_seg, buffer_off);

        if max_chars == 0 {
            sys.write_u8(buffer_seg, buffer_off.wrapping_add(1), 0);
            set_carry_flag(sys, false);
            return;
        }

        sys.console_input.active = true;
        sys.console_input.buffer_seg = buffer_seg;
        sys.console_input.buffer_off = buffer_off;
        sys.console_input.max_chars = max_chars;
        sys.console_input.current_count = 0;

        sys.write_u8(buffer_seg, buffer_off.wrapping_add(1), 0);
    }

    if let Some(key) = sys.machine.keyboard.pop_key() {
        let ascii = (key & 0xFF) as u8;

        match ascii {
            0x0D => {
                sys.console_input.active = false;
                sys.write_u8(
                    sys.console_input.buffer_seg,
                    sys.console_input.buffer_off.wrapping_add(1),
                    sys.console_input.current_count,
                );
                set_carry_flag(sys, false);
                println!();
                std::io::stdout().flush().ok();
                return;
            }
            0x08 => {
                if sys.console_input.current_count > 0 {
                    sys.console_input.current_count -= 1;
                    tty_write(sys, 8);
                    tty_write(sys, b' ');
                    tty_write(sys, 8);
                    std::io::stdout().flush().ok();
                }
            }
            0x1B => {
                sys.console_input.current_count = 0;
                log::debug!("Buffered input: Esc pressed, line cleared");
            }
            0x03 => {
                sys.console_input.active = false;
                sys.console_input.current_count = 0;
                log::info!("Buffered input: Ctrl+C, raising INT 23h");
                sys.raise_interrupt(0x23);
                return;
            }
            0x0A => {}
            _ => {
                if sys.console_input.current_count < sys.console_input.max_chars {
                    let pos = 2 + sys.console_input.current_count as u16;
                    sys.write_u8(
                        sys.console_input.buffer_seg,
                        sys.console_input.buffer_off.wrapping_add(pos),
                        ascii,
                    );
                    sys.console_input.current_count += 1;
                    if ascii >= 0x20 {
                        tty_write(sys, ascii);
                    }
                }
            }
        }
        if sys.console_input.active {
            sys.suspend_for_key(0x21);
        }
    } else {
        sys.suspend_for_key(0x21);
    }
}
