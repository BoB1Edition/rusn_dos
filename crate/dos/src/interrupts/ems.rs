// Ver: 3 File: crate/dos/src/interrupts/ems.rs
use x86::flags;

use crate::system::DosSystem;

pub fn handle_int67(sys: &mut DosSystem) {
    if !sys.machine_config.ems {
        let mut f = sys.cpu.registers.flags();
        f |= flags::CF;
        sys.cpu.registers.set_flags(f);
        sys.cpu.registers.set_ah(0x84);
        return;
    }
    match sys.cpu.registers.ah() {
        0x40 => {
            // Get Status
            sys.cpu.registers.set_ah(0x00);
        }
        0x41 => {
            // Get Page Frame Segment
            sys.cpu.registers.set_bx(sys.ems.page_frame_segment);
            sys.cpu.registers.set_ah(0x00);
        }
        0x42 => {
            // Map/Unmap Page (AL=log page, BX=handle, DX=phys page)
            let handle = sys.cpu.registers.bx();
            if handle == 0 || !sys.ems.handles.iter().any(|&(h, _)| h == handle) {
                sys.cpu.registers.set_ah(0x81); // Invalid handle
            } else {
                sys.cpu.registers.set_ah(0x00);
            }
        }
        0x43 => {
            // Get Unallocated Pages
            sys.cpu.registers.set_bx(sys.ems.free_pages);
            sys.cpu.registers.set_ah(0x00);
        }
        0x44 => {
            // Allocate Pages (BX = requested)
            let requested = sys.cpu.registers.bx();
            if requested == 0 {
                sys.cpu.registers.set_ah(0x81);
            } else if requested > sys.ems.free_pages {
                sys.cpu.registers.set_ah(0x83); // Out of pages
            } else {
                sys.ems.free_pages -= requested;
                let handle = sys.ems.next_handle;
                sys.ems.next_handle += 1;
                sys.ems.handles.push((handle, requested));
                sys.cpu.registers.set_dx(handle);
                sys.cpu.registers.set_ah(0x00);
            }
        }
        0x45 => {
            // Deallocate Pages (DX = handle)
            let handle = sys.cpu.registers.dx();
            if let Some(pos) = sys.ems.handles.iter().position(|&(h, _)| h == handle) {
                let pages = sys.ems.handles.remove(pos).1;
                sys.ems.free_pages += pages;
                sys.cpu.registers.set_ah(0x00);
            } else {
                sys.cpu.registers.set_ah(0x81); // Invalid handle
            }
        }
        0x49 => {
            // Get Handle Pages (DX = handle)
            let handle = sys.cpu.registers.dx();
            if let Some(&(_, pages)) = sys.ems.handles.iter().find(|&&(h, _)| h == handle) {
                sys.cpu.registers.set_bx(pages);
                sys.cpu.registers.set_ah(0x00);
            } else {
                sys.cpu.registers.set_ah(0x81);
            }
        }
        0x4B => {
            // Get Version
            sys.cpu.registers.set_ax(0x0400); // EMS 4.00
            sys.cpu.registers.set_ah(0x00);
        }
        0x4D => {
            // Reinitialize EMM
            sys.ems.free_pages = sys.ems.total_pages;
            sys.ems.next_handle = 1;
            sys.ems.handles.clear();
            sys.cpu.registers.set_ah(0x00);
        }
        0x56 => {
            // Allocate & Map (EMM 4.0+)
            let requested = sys.cpu.registers.bx();
            if requested == 0 || requested > sys.ems.free_pages {
                sys.cpu.registers.set_ah(0x83);
            } else {
                sys.ems.free_pages -= requested;
                let handle = sys.ems.next_handle;
                sys.ems.next_handle += 1;
                sys.ems.handles.push((handle, requested));
                sys.cpu.registers.set_dx(handle);
                sys.cpu.registers.set_ah(0x00);
            }
        }
        // Заглушки для менее критичных функций
        0x46 | 0x47 | 0x48 | 0x4E | 0x53 | 0x54 | 0x57 => {
            sys.cpu.registers.set_ah(0x00);
        }
        0xDE => {
            // DPMI / VCPI Installation Check (AX=DE00h)
            let mut f = sys.cpu.registers.flags();
            f |= flags::CF; // Not Installed
            sys.cpu.registers.set_flags(f);
            log::info!("INT 67h / AX=DE00h: DPMI Host Check -> Not Installed");
        }
        _ => {
            log::warn!(
                "Unsupported EMS INT 67h / AH={:02X}",
                sys.cpu.registers.ah()
            );
            sys.cpu.registers.set_ah(0x80); // Internal error / unsupported
        }
    }
}
