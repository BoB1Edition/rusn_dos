use crate::filesystem::FoundFile;
use crate::system::DosSystem;

use super::misc::set_carry_flag;

pub fn find_first(sys: &mut DosSystem) {
    let path = sys.filesystem.extract_filename(
        sys.cpu.registers.ds(),
        sys.cpu.registers.dx(),
        &sys.machine,
    );
    let dta_addr = get_dta_addr(sys);

    match sys.filesystem.find_first(&path, dta_addr) {
        Ok(Some(found)) => {
            write_dta(sys, dta_addr, &found);
            set_carry_flag(sys, false);
        }
        Ok(None) => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(18);
        }
        Err(e) => {
            log::error!("Find first failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(2);
        }
    }
}

pub fn find_next(sys: &mut DosSystem) {
    let dta_addr = get_dta_addr(sys);

    match sys.filesystem.find_next(dta_addr) {
        Ok(Some(found)) => {
            write_dta(sys, dta_addr, &found);
            set_carry_flag(sys, false);
        }
        Ok(None) => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(18);
        }
        Err(e) => {
            log::error!("Find next failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(18);
        }
    }
}

fn get_dta_addr(sys: &DosSystem) -> u32 {
    sys.get_dta_addr()
}

fn write_dta(sys: &mut DosSystem, dta_addr: u32, found: &FoundFile) {
    sys.write_phys_u8(dta_addr + 0x15, found.attr);
    sys.write_phys_u16(dta_addr + 0x16, found.time);
    sys.write_phys_u16(dta_addr + 0x18, found.date);
    sys.write_phys_u32(dta_addr + 0x1A, found.size);

    let name_bytes = found.name_83.as_bytes();
    for (i, &b) in name_bytes.iter().take(12).enumerate() {
        sys.write_phys_u8(dta_addr + 0x1E + i as u32, b);
    }
    sys.write_phys_u8(dta_addr + 0x1E + name_bytes.len().min(12) as u32, 0);
}
