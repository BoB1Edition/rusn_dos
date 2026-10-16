// Ver: 2 File: crate/dos/src/interrupts/mod.rs

pub mod bios;
pub mod dos;
pub mod ems;

use bus::Machine;

use crate::system::DosSystem;

/// Маршрутизация внутренних векторов прерываний на обработчики.
pub fn dispatch(sys: &mut DosSystem, vector: u8) {
    match vector {
        0x08 => bios::handle_int08(sys),
        0x09 => {
            sys.machine.out_port(0x20, 0x20);
            log::debug!("IRQ1 (Keyboard) handled");
        }
        0x10 => bios::handle_int10(sys),
        0x12 => bios::handle_int12(sys),
        0x15 => bios::handle_int15(sys),
        0x16 => bios::handle_int16(sys),
        0x1A => bios::handle_int1a(sys),
        0x1C => log::info!("INT 1Ch (User Timer) called"),
        0x20 => sys.cpu.halted = true,
        0x21 => dos::handle_int21(sys),
        0x2F => dos::handle_int2f(sys),
        0x67 => ems::handle_int67(sys),
        _ => {
            log::error!(
                "Unhandled internal interrupt INT {:02X} at CS:IP={:04X}:{:04X}",
                vector,
                sys.cpu.registers.cs(),
                sys.cpu.registers.ip()
            );
            sys.cpu.halted = true;
        }
    }
}
