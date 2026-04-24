// Ver: 2
use crate::{DosMachine, flags};

pub fn handle_int67(machine: &mut DosMachine) {
    match machine.registers.ah() {
        0x40 => {
            // Get Status
            machine.registers.set_ah(0x00);
        }
        0x41 => {
            // Get Page Frame Segment
            machine.registers.set_bx(machine.ems_page_frame_segment);
            machine.registers.set_ah(0x00);
        }
        0x42 => {
            // Map/Unmap Page (AL=log page, BX=handle, DX=phys page)
            // Простейшая валидация хендла
            let handle = machine.registers.bx();
            if handle == 0 || !machine.ems_handles.iter().any(|&(h, _)| h == handle) {
                machine.registers.set_ah(0x81); // Invalid handle
            } else {
                machine.registers.set_ah(0x00);
            }
        }
        0x43 => {
            // Get Unallocated Pages
            machine.registers.set_bx(machine.ems_free_pages);
            machine.registers.set_ah(0x00);
        }
        0x44 => {
            // Allocate Pages (BX = requested)
            let requested = machine.registers.bx();
            if requested == 0 {
                machine.registers.set_ah(0x81);
            } else if requested > machine.ems_free_pages {
                machine.registers.set_ah(0x83); // Out of pages
            } else {
                machine.ems_free_pages -= requested;
                let handle = machine.ems_next_handle;
                machine.ems_next_handle += 1;
                machine.ems_handles.push((handle, requested));
                machine.registers.set_dx(handle);
                machine.registers.set_ah(0x00);
            }
        }
        0x45 => {
            // Deallocate Pages (DX = handle)
            let handle = machine.registers.dx();
            if let Some(pos) = machine.ems_handles.iter().position(|&(h, _)| h == handle) {
                let pages = machine.ems_handles.remove(pos).1;
                machine.ems_free_pages += pages;
                machine.registers.set_ah(0x00);
            } else {
                machine.registers.set_ah(0x81); // Invalid handle
            }
        }
        0x49 => {
            // Get Handle Pages (DX = handle)
            let handle = machine.registers.dx();
            if let Some(&(_, pages)) = machine.ems_handles.iter().find(|&&(h, _)| h == handle) {
                machine.registers.set_bx(pages);
                machine.registers.set_ah(0x00);
            } else {
                machine.registers.set_ah(0x81);
            }
        }
        0x4B => {
            // Get Version
            machine.registers.set_ax(0x0400); // EMS 4.00
            machine.registers.set_ah(0x00);
        }
        0x4D => {
            // Reinitialize EMM
            machine.ems_free_pages = machine.ems_total_pages;
            machine.ems_next_handle = 1;
            machine.ems_handles.clear();
            machine.registers.set_ah(0x00);
        }
        0x56 => {
            // Allocate & Map (EMM 4.0+)
            let requested = machine.registers.bx();
            if requested == 0 || requested > machine.ems_free_pages {
                machine.registers.set_ah(0x83);
            } else {
                machine.ems_free_pages -= requested;
                let handle = machine.ems_next_handle;
                machine.ems_next_handle += 1;
                machine.ems_handles.push((handle, requested));
                machine.registers.set_dx(handle);
                machine.registers.set_ah(0x00);
            }
        }
        // Заглушки для менее критичных функций
        0x46 | 0x47 | 0x48 | 0x4E | 0x53 | 0x54 | 0x57 => {
            machine.registers.set_ah(0x00);
        }
        0xDE => {
            // DPMI / VCPI Installation Check (AX=DE00h)
            // Мы не поддерживаем защищенный режим, поэтому возвращаем "Not Installed" (CF=1).
            // Это стандартное поведение для Real Mode окружений без DPMI.
            let mut f = machine.registers.flags();
            f |= flags::CF; // Установить флаг Carry
            machine.registers.set_flags(f);

            if machine.registers.al() == 0 {
                log::info!("INT 67h / AX=DE00h: DPMI Host Check -> Not Installed");
            }
        }
        _ => {
            log::warn!(
                "Unsupported EMS INT 67h / AH={:02X}",
                machine.registers.ah()
            );
            machine.registers.set_ah(0x80); // Internal error / unsupported
        }
    }
}
