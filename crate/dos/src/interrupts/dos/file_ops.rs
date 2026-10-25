// Ver: 1 File: ./crate/dos/src/interrupts/dos/file_ops.rs
use crate::filesystem::DeviceKind;
use crate::interrupts::bios::tty_write;
use crate::system::DosSystem;

use super::misc::set_carry_flag;

pub fn open_file(sys: &mut DosSystem) {
    let filename = sys.filesystem.extract_filename(
        sys.cpu.registers.ds(),
        sys.cpu.registers.dx(),
        &sys.machine,
    );
    let access_mode = sys.cpu.registers.al() & 0x07;

    log::info!(
        "INT 21h / AH=3Dh: Open file '{}' (mode={})",
        filename,
        match access_mode {
            0 => "read-only",
            1 => "write-only",
            2 => "read-write",
            _ => "unknown",
        }
    );
    match sys.filesystem.open_file(&filename, access_mode) {
        Ok(handle) => {
            set_carry_flag(sys, false);
            sys.cpu.registers.set_ax(handle);
            log::info!("File opened successfully, handle={}", handle);
        }
        Err(e) => {
            log::error!("Open failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(2);
        }
    }
}

pub fn close_file(sys: &mut DosSystem) {
    let handle = sys.cpu.registers.bx();
    log::info!("INT 21h / AH=3Eh: Close file handle {}", handle);

    match sys.filesystem.close_file(handle) {
        Ok(_) => {
            set_carry_flag(sys, false);
        }
        Err(e) => {
            log::warn!("Close failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(6);
        }
    }
}

pub fn read_file(sys: &mut DosSystem) {
    let handle = sys.cpu.registers.bx();
    let count = sys.cpu.registers.cx();
    let buffer_seg = sys.cpu.registers.ds();
    let buffer_off = sys.cpu.registers.dx();

    if sys.filesystem.device_of(handle) == Some(DeviceKind::Con) {
        if let Some(key) = sys.machine.keyboard.pop_key() {
            let ascii = (key & 0xFF) as u8;
            sys.write_u8(buffer_seg, buffer_off, ascii);
            if ascii >= 0x20 || ascii == 0x0D {
                tty_write(sys, ascii);
            }
            sys.cpu.registers.set_ax(1);
            set_carry_flag(sys, false);
        } else {
            sys.suspend_for_key(0x21);
        }
        return;
    }

    let mut buffer = vec![0u8; count as usize];

    match sys.filesystem.read_file(handle, &mut buffer) {
        Ok(bytes_read) => {
            for (i, &byte) in buffer.iter().take(bytes_read as usize).enumerate() {
                sys.write_u8(buffer_seg, buffer_off.wrapping_add(i as u16), byte);
            }
            set_carry_flag(sys, false);
            sys.cpu.registers.set_ax(bytes_read);
        }
        Err(e) => {
            log::error!("Read failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(1);
        }
    }
}

pub fn write_file(sys: &mut DosSystem) {
    let handle = sys.cpu.registers.bx();
    let count = sys.cpu.registers.cx();
    let buffer_seg = sys.cpu.registers.ds();
    let buffer_off = sys.cpu.registers.dx();

    let mut buffer = vec![0u8; count as usize];
    for i in 0..count as usize {
        buffer[i] = sys.read_u8(buffer_seg, buffer_off.wrapping_add(i as u16));
    }

    match sys.filesystem.write_file(handle, &buffer) {
        Ok(bytes_written) => {
            set_carry_flag(sys, false);
            sys.cpu.registers.set_ax(bytes_written);
        }
        Err(e) => {
            log::error!("Write failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(1);
        }
    }
}

pub fn seek_file(sys: &mut DosSystem) {
    let handle = sys.cpu.registers.bx();
    let offset_high = sys.cpu.registers.cx() as i32;
    let offset_low = sys.cpu.registers.dx() as i32;
    let origin = sys.cpu.registers.al();

    let offset = ((offset_high as i32) << 16) | (offset_low as i32);

    match sys.filesystem.seek_file(handle, offset, origin) {
        Ok(new_pos) => {
            set_carry_flag(sys, false);
            sys.cpu.registers.set_dx((new_pos >> 16) as u16);
            sys.cpu.registers.set_ax((new_pos & 0xFFFF) as u16);
        }
        Err(e) => {
            log::error!("Seek failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(1);
        }
    }
}

pub fn create_file(sys: &mut DosSystem) {
    let path = sys.filesystem.extract_filename(
        sys.cpu.registers.ds(),
        sys.cpu.registers.dx(),
        &sys.machine,
    );
    let attrs = sys.cpu.registers.cx();

    match sys.filesystem.create_file(&path, attrs) {
        Ok(handle) => {
            set_carry_flag(sys, false);
            sys.cpu.registers.set_ax(handle);
        }
        Err(e) => {
            log::error!("Create file failed: {}", e);
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(5);
        }
    }
}

pub fn delete_file(sys: &mut DosSystem) {
    let path = sys.filesystem.extract_filename(
        sys.cpu.registers.ds(),
        sys.cpu.registers.dx(),
        &sys.machine,
    );
    match sys.filesystem.delete_file(&path) {
        Ok(_) => set_carry_flag(sys, false),
        Err(e) => {
            log::error!("Delete file failed: {}", e);
            set_carry_flag(sys, true);
            if e.contains("Access denied") {
                sys.cpu.registers.set_ax(5);
            } else {
                sys.cpu.registers.set_ax(2);
            }
        }
    }
}

pub fn file_attributes(sys: &mut DosSystem) {
    let al = sys.cpu.registers.al();
    let filename = sys.filesystem.extract_filename(
        sys.cpu.registers.ds(),
        sys.cpu.registers.dx(),
        &sys.machine,
    );
    if crate::filesystem::resolve_device(&filename).is_some() {
        sys.cpu.registers.set_cx(0x20);
        set_carry_flag(sys, false);
        return;
    }

    match al {
        0x00 => match sys.filesystem.resolve_path(&filename) {
            Ok(path) if path.exists() => {
                sys.cpu.registers.set_cx(0x20);
                set_carry_flag(sys, false);
            }
            _ => {
                set_carry_flag(sys, true);
                sys.cpu.registers.set_ax(2);
            }
        },
        0x01 => match sys.filesystem.resolve_path(&filename) {
            Ok(path) if path.exists() => {
                set_carry_flag(sys, false);
            }
            _ => {
                set_carry_flag(sys, true);
                sys.cpu.registers.set_ax(2);
            }
        },
        _ => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(1);
        }
    }
}
