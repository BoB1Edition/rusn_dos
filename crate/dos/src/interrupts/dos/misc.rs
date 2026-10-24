use bus::Machine;
use x86::flags;

use crate::system::DosSystem;

pub fn set_carry_flag(sys: &mut DosSystem, value: bool) {
    let mut f = sys.cpu.registers.flags();
    if value {
        f |= flags::CF;
    } else {
        f &= !flags::CF;
    }
    sys.cpu.registers.set_flags(f);
}

pub fn set_dta(sys: &mut DosSystem) {
    sys.dta_segment = sys.cpu.registers.ds();
    sys.dta_offset = sys.cpu.registers.dx();
    set_carry_flag(sys, false);
    log::info!(
        "INT 21h / AH=1Ah: Set DTA -> {:04X}:{:04X}",
        sys.dta_segment,
        sys.dta_offset
    );
}

pub fn get_interrupt_vector(sys: &mut DosSystem) {
    let vector = sys.cpu.registers.al();
    let addr = (vector as u32) * 4;
    let ip = sys.machine.read_mem_u16(addr);
    let cs = sys.machine.read_mem_u16(addr + 2);
    sys.cpu.registers.set_es(cs);
    sys.cpu.registers.set_bx(ip);
    set_carry_flag(sys, false);
    log::info!(
        "INT 21h / AH=35h: Get interrupt vector {:02X}h -> {:04X}:{:04X}",
        vector,
        cs,
        ip
    );
}

pub fn set_interrupt_vector(sys: &mut DosSystem) {
    let vector = sys.cpu.registers.al();
    let handler_ip = sys.cpu.registers.dx();
    let handler_cs = sys.cpu.registers.ds();

    let addr = (vector as u32) * 4;
    sys.machine.write_mem_u16(addr, handler_ip);
    sys.machine.write_mem_u16(addr + 2, handler_cs);

    log::info!(
        "INT 21h / AH=25h: Set interrupt vector {:02X}h -> {:04X}:{:04X}",
        vector,
        handler_cs,
        handler_ip
    );
}

pub fn ioctl(sys: &mut DosSystem) {
    let al = sys.cpu.registers.al();
    let bx = sys.cpu.registers.bx();

    match al {
        0x00 => {
            if sys.filesystem.device_of(bx).is_some() {
                sys.cpu.registers.set_dx(0x80D3);
                set_carry_flag(sys, false);
            } else {
                let drive = sys.filesystem.get_current_drive();
                let drive_num = (drive as u8 - b'A') as u16;
                sys.cpu.registers.set_dx(drive_num);
                set_carry_flag(sys, false);
            }
        }
        _ => {
            log::warn!("Unsupported IOCTL subfunction AL={:02X}", al);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x0001);
        }
    }
}
