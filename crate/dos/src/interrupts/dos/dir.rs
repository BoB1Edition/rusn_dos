use crate::system::DosSystem;

use super::misc::set_carry_flag;

pub fn set_current_directory(sys: &mut DosSystem) {
    let path = sys.filesystem.extract_filename(
        sys.cpu.registers.ds(),
        sys.cpu.registers.dx(),
        &sys.machine,
    );

    let (drive, dir_path) = if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        (
            path.chars().next().unwrap().to_ascii_uppercase(),
            &path[2..],
        )
    } else {
        (sys.filesystem.get_current_drive(), path.as_str())
    };

    match sys.filesystem.set_current_directory(drive, dir_path) {
        Ok(_) => set_carry_flag(sys, false),
        Err(e) => {
            log::error!("CHDIR failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(3);
        }
    }
}

pub fn get_current_directory(sys: &mut DosSystem) {
    let dl = sys.cpu.registers.dl();
    let drive_letter = if dl == 0 {
        'C'
    } else {
        (b'A' + dl - 1) as char
    };

    let dir = sys
        .filesystem
        .get_current_directory(drive_letter)
        .unwrap_or("")
        .to_string();

    let seg = sys.cpu.registers.ds();
    let mut off = sys.cpu.registers.si();
    for &byte in dir.as_bytes() {
        sys.write_u8(seg, off, byte);
        off = off.wrapping_add(1);
    }
    sys.write_u8(seg, off, 0);

    set_carry_flag(sys, false);
    log::info!(
        "INT 21h / AH=47h: Get current directory for drive {}: -> '{}'",
        drive_letter,
        dir
    );
}

pub fn get_current_drive(sys: &mut DosSystem) {
    sys.cpu.registers.set_al(2);
    log::info!("INT 21h / AH=19h: Get current drive -> C:");
}
