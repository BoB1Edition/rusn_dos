// Ver: 1 File: ./libs/dos_core/src/mcb.rs
//! DOS Memory Control Block (MCB) allocator.
//! Real-mode paragraph-based memory manager.

use crate::{DosMachine, consts::{MCB_SIGNATURE_LAST, MCB_SIGNATURE_NON_LAST}};

/// Структура MCB (16 байт в памяти)
#[derive(Debug, Clone, Copy)]
pub struct MCB {
    pub signature: u8,
    pub owner: u16, // PSP владельца, 0 = свободен
    pub size: u16,  // размер в параграфах (не включая сам MCB)
}

impl MCB {
    /// Читает MCB из памяти по сегменту
    pub fn read(machine: &DosMachine, segment: u16) -> Self {
        let phys = (segment as u32) << 4;
        Self {
            signature: machine.read_phys_u8(phys),
            owner: machine.read_phys_u16(phys.wrapping_add(1)),
            size: machine.read_phys_u16(phys.wrapping_add(3)),
        }
    }

    /// Записывает MCB в память по сегменту
    pub fn write(&self, machine: &mut DosMachine, segment: u16) {
        let phys = (segment as u32) << 4;
        machine.write_phys_u8(phys, self.signature);
        machine.write_phys_u16(phys.wrapping_add(1), self.owner);
        machine.write_phys_u16(phys.wrapping_add(3), self.size);
    }
}

/// Инициализирует карту памяти, создавая первый MCB, охватывающий всю доступную память.
pub fn init_memory_map(machine: &mut DosMachine, first_mcb_segment: u16) {
    let total_paragraphs = (machine.memory.len() / 16) as u32;
    let available = total_paragraphs.saturating_sub(first_mcb_segment as u32);
    log::debug!("total_paragraphs: {}, available: {}, len: {}", total_paragraphs, available, machine.memory.len());
    if available < 2 {
        log::error!("Not enough memory for MCB chain");
        return;
    }
    // Первый MCB — свободный, последний
    let mcb = MCB {
        signature: MCB_SIGNATURE_LAST,
        owner: 0,
        size: (available - 1) as u16, // минус сам MCB
    };
    mcb.write(machine, first_mcb_segment);
}

/// Выделяет блок памяти указанного размера (в параграфах).
/// Возвращает сегмент выделенного блока (адрес после MCB) или None, если не хватает памяти.
pub fn allocate(machine: &mut DosMachine, first_seg: u16, paragraphs: u16) -> Option<u16> {
    let mut seg = first_seg;
    loop {
        let mcb = MCB::read(machine, seg);
        if mcb.owner == 0 && mcb.size >= paragraphs {
            // если блок больше, чем нужно, и можно разрезать
            if mcb.size > paragraphs + 1 {
                let rest_seg = seg + 1 + paragraphs;
                let rest_size = mcb.size - paragraphs - 1;
                // обновляем текущий MCB
                MCB {
                    signature: MCB_SIGNATURE_NON_LAST,
                    owner: 0x0001, // временный владелец (потом исправится на реальный PSP)
                    size: paragraphs,
                }.write(machine, seg);
                // создаём свободный остаток
                MCB {
                    signature: mcb.signature, // наследуем флаг последнего
                    owner: 0,
                    size: rest_size,
                }.write(machine, rest_seg);
                return Some(seg + 1);
            } else {
                // используем весь блок
                MCB {
                    owner: 0x0001,
                    ..mcb
                }.write(machine, seg);
                return Some(seg + 1);
            }
        }
        if mcb.signature == MCB_SIGNATURE_LAST {
            break;
        }
        seg += 1 + mcb.size;
    }
    None
}

/// Освобождает блок памяти по сегменту данных (MCB = segment - 1).
/// Возвращает Ok(()) или Err(код ошибки DOS).
pub fn free(machine: &mut DosMachine, first_seg: u16, data_segment: u16) -> Result<(), u16> {
    let mcb_seg = data_segment - 1;
    let mut mcb = MCB::read(machine, mcb_seg);
    if mcb.owner == 0 {
        return Err(0x09); // уже свободен или повреждён
    }
    mcb.owner = 0;
    mcb.write(machine, mcb_seg);
    coalesce(machine, first_seg);
    Ok(())
}

/// Изменяет размер блока памяти.
/// Возвращает Ok(максимальный размер) или Err(код ошибки).
pub fn modify(machine: &mut DosMachine, first_seg: u16, data_segment: u16, new_paragraphs: u16) -> Result<u16, u16> {
    let mcb_seg = data_segment - 1;
    let mcb = MCB::read(machine, mcb_seg);
    if mcb.owner == 0 {
        return Err(0x09);
    }

    if mcb.size >= new_paragraphs {
        // уменьшение
        let excess = mcb.size - new_paragraphs;
        if excess > 0 {
            // создаём новый свободный MCB после уменьшенного блока
            let next_seg = mcb_seg + 1 + new_paragraphs;
            let next_sig = if mcb.signature == MCB_SIGNATURE_LAST { MCB_SIGNATURE_LAST } else { MCB_SIGNATURE_NON_LAST };
            MCB { size: new_paragraphs, ..mcb }.write(machine, mcb_seg);
            MCB { signature: next_sig, owner: 0, size: excess - 1 }.write(machine, next_seg);
        }
        coalesce(machine, first_seg);
        Ok(0) // успех
    } else {
        // попытка увеличить
        let next_seg = mcb_seg + 1 + mcb.size;
        let next = MCB::read(machine, next_seg);
        if next.owner == 0 && (mcb.size + 1 + next.size) >= new_paragraphs {
            let combined = mcb.size + 1 + next.size;
            let rest = combined - new_paragraphs;
            let new_sig = if rest > 0 { MCB_SIGNATURE_NON_LAST } else { mcb.signature };
            MCB { size: new_paragraphs, signature: new_sig, ..mcb }.write(machine, mcb_seg);
            if rest > 0 {
                let new_next = mcb_seg + 1 + new_paragraphs;
                MCB { signature: mcb.signature, owner: 0, size: rest - 1 }.write(machine, new_next);
            }
            coalesce(machine, first_seg);
            Ok(0)
        } else {
            // возвращаем ошибку с максимальным доступным размером
            let max = max_available_at(machine, first_seg, data_segment);
            Err(max) // код ошибки будет установлен отдельно, в BX вернём max
        }
    }
}

/// Возвращает размер наибольшего свободного блока (в параграфах).
pub fn max_available(machine: &DosMachine, first_seg: u16) -> u16 {
    let mut seg = first_seg;
    let mut max = 0;
    loop {
        let mcb = MCB::read(machine, seg);
        if mcb.owner == 0 && mcb.size > max {
            max = mcb.size;
        }
        if mcb.signature == MCB_SIGNATURE_LAST {
            break;
        }
        seg += 1 + mcb.size;
    }
    max
}

/// Возвращает максимальный размер, до которого можно увеличить блок (включая следующий свободный).
pub fn max_available_at(machine: &DosMachine, first_seg: u16, data_segment: u16) -> u16 {
    let mcb_seg = data_segment - 1;
    let mcb = MCB::read(machine, mcb_seg);
    let next_seg = mcb_seg + 1 + mcb.size;
    let next = MCB::read(machine, next_seg);
    if next.owner == 0 {
        mcb.size + 1 + next.size
    } else {
        mcb.size
    }
}

/// Объединяет смежные свободные блоки в цепочке, начиная с first_seg.
fn coalesce(machine: &mut DosMachine, first_seg: u16) {
    let mut seg = first_seg;
    loop {
        let mcb = MCB::read(machine, seg);
        if mcb.signature == MCB_SIGNATURE_LAST {
            break;
        }
        let next_seg = seg + 1 + mcb.size;
        let next = MCB::read(machine, next_seg);
        if mcb.owner == 0 && next.owner == 0 {
            // объединяем
            let new_size = mcb.size + 1 + next.size;
            MCB {
                size: new_size,
                signature: next.signature,
                ..mcb
            }.write(machine, seg);
            // не сдвигаем seg, остаёмся на этом же месте (могло появиться ещё свободное место)
        } else {
            seg = next_seg;
        }
    }
}