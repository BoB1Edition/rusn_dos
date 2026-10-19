// Ver: 4 File: crate/dos/src/mcb.rs
//! DOS Memory Control Block (MCB) allocator.
//! Real-mode paragraph-based memory manager.
use bus::Machine;
use crate::consts::{MCB_SIGNATURE_LAST, MCB_SIGNATURE_NON_LAST};

/// Структура MCB (16 байт в памяти)
#[derive(Debug, Clone, Copy)]
pub struct MCB {
    pub signature: u8,
    pub owner: u16, // PSP владельца, 0 = свободен
    pub size: u16,  // размер в параграфах (не включая сам MCB)
}

impl MCB {
    /// Читает MCB из памяти по сегменту
    pub fn read(machine: &dyn Machine, segment: u16) -> Self {
        let phys = (segment as u32) << 4;
        Self {
            signature: machine.read_mem_u8(phys),
            owner: machine.read_mem_u16(phys.wrapping_add(1)),
            size: machine.read_mem_u16(phys.wrapping_add(3)),
        }
    }

    /// Записывает MCB в память по сегменту
    pub fn write(&self, machine: &mut dyn Machine, segment: u16) {
        let phys = (segment as u32) << 4;
        machine.write_mem_u8(phys, self.signature);
        machine.write_mem_u16(phys.wrapping_add(1), self.owner);
        machine.write_mem_u16(phys.wrapping_add(3), self.size);
    }
}

/// Инициализирует карту памяти, создавая первый MCB, охватывающий всю доступную память.
pub fn init_memory_map(machine: &mut dyn Machine, first_mcb_segment: u16) {
    const MAX_CONVENTIONAL_SEGMENT: u16 = 0xF000; // сразу за 640 КБ начинается видеопамять
    let available = MAX_CONVENTIONAL_SEGMENT.saturating_sub(first_mcb_segment);
    log::debug!("available: {}", available);
    
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
pub fn allocate(machine: &mut dyn Machine, first_seg: u16, paragraphs: u16) -> Option<u16> {
    let mut seg = first_seg;
    loop {
        let mcb = MCB::read(machine, seg);
        
        if mcb.owner == 0 && mcb.size >= paragraphs {
            // если блок больше, чем нужно, и можно разрезать (нужен минимум 1 параграф для нового MCB + 1 для данных)
            if mcb.size > paragraphs + 1 {
                let rest_seg = seg.wrapping_add(1).wrapping_add(paragraphs);
                let rest_size = mcb.size.wrapping_sub(paragraphs).wrapping_sub(1);
                
                // обновляем текущий MCB
                MCB {
                    signature: MCB_SIGNATURE_NON_LAST,
                    owner: 0x0001, // временный владелец (потом исправится на реальный PSP)
                    size: paragraphs,
                }
                .write(machine, seg);
                
                // создаём свободный остаток
                MCB {
                    signature: mcb.signature, // наследуем флаг последнего
                    owner: 0,
                    size: rest_size,
                }
                .write(machine, rest_seg);
                
                return Some(seg.wrapping_add(1));
            } else {
                // используем весь блок
                MCB {
                    owner: 0x0001,
                    ..mcb
                }
                .write(machine, seg);
                return Some(seg.wrapping_add(1));
            }
        }
        
        if mcb.signature == MCB_SIGNATURE_LAST {
            break;
        }
        
        // ✅ ЗАЩИТА: безопасное сложение и проверка на зацикливание
        let next_seg = seg.wrapping_add(1).wrapping_add(mcb.size);
        if next_seg <= seg {
            log::warn!("MCB chain corrupted during allocate at segment {:04X}, breaking", seg);
            break;
        }
        seg = next_seg;
    }
    None
}

/// Освобождает блок памяти по сегменту данных (MCB = segment - 1).
/// Возвращает Ok(()) или Err(код ошибки DOS).
pub fn free(machine: &mut dyn Machine, first_seg: u16, data_segment: u16) -> Result<(), u16> {
    // ✅ ЗАЩИТА: сегмент 0 не может иметь MCB
    if data_segment == 0 {
        return Err(0x09);
    }
    
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
pub fn modify(
    machine: &mut dyn Machine,
    first_seg: u16,
    data_segment: u16,
    new_paragraphs: u16,
) -> Result<u16, u16> {
    if data_segment == 0 {
        return Err(0x09);
    }
    
    let mcb_seg = data_segment - 1;
    let mcb = MCB::read(machine, mcb_seg);
    
    if mcb.owner == 0 {
        return Err(0x09);
    }
    
    if mcb.size >= new_paragraphs {
        // уменьшение
        let excess = mcb.size.wrapping_sub(new_paragraphs);
        
        // ✅ ИСПРАВЛЕНО: разбиваем блок только если остатка хватает на новый MCB (1) + данные (>=1)
        if excess > 1 {
            let next_seg = mcb_seg.wrapping_add(1).wrapping_add(new_paragraphs);
            let next_sig = if mcb.signature == MCB_SIGNATURE_LAST {
                MCB_SIGNATURE_LAST
            } else {
                MCB_SIGNATURE_NON_LAST
            };
            
            MCB {
                size: new_paragraphs,
                ..mcb
            }
            .write(machine, mcb_seg);
            
            MCB {
                signature: next_sig,
                owner: 0,
                size: excess.wrapping_sub(1),
            }
            .write(machine, next_seg);
        }
        coalesce(machine, first_seg);
        Ok(0) // успех
    } else {
        // попытка увеличить
        let next_seg = mcb_seg.wrapping_add(1).wrapping_add(mcb.size);
        
        // ✅ ЗАЩИТА: если next_seg <= mcb_seg, значит переполнение или повреждение
        if next_seg <= mcb_seg {
            return Err(mcb.size); 
        }
        
        let next = MCB::read(machine, next_seg);
        let combined = mcb.size.wrapping_add(1).wrapping_add(next.size);
        
        if next.owner == 0 && combined >= new_paragraphs {
            let rest = combined.wrapping_sub(new_paragraphs);
            let (final_size, new_sig) = if rest > 1 {
                let new_next = mcb_seg.wrapping_add(1).wrapping_add(new_paragraphs);
                MCB {
                    signature: MCB_SIGNATURE_NON_LAST,
                    owner: 0,
                    size: rest.wrapping_sub(1),
                }
                .write(machine, new_next);
                (new_paragraphs, MCB_SIGNATURE_NON_LAST)
            } else {
                (combined, next.signature)
            };

            MCB {
                size: final_size,
                signature: new_sig,
                ..mcb
            }
            .write(machine, mcb_seg);
            
            coalesce(machine, first_seg);
            Ok(0)
        } else {
            let max = max_available_at(machine, first_seg, data_segment);
            Err(max)
        }
    }
}

/// Возвращает размер наибольшего свободного блока (в параграфах).
pub fn max_available(machine: &dyn Machine, first_seg: u16) -> u16 {
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
        
        let next_seg = seg.wrapping_add(1).wrapping_add(mcb.size);
        if next_seg <= seg {
            break; // защита от зацикливания
        }
        seg = next_seg;
    }
    max
}

/// Возвращает максимальный размер, до которого можно увеличить блок (включая следующий свободный).
pub fn max_available_at(machine: &dyn Machine, first_seg: u16, data_segment: u16) -> u16 {
    let _ = first_seg;
    if data_segment == 0 {
        return 0;
        }
    let mcb_seg = data_segment - 1;
    let mcb = MCB::read(machine, mcb_seg);
    
    let next_seg = mcb_seg.wrapping_add(1).wrapping_add(mcb.size);
    if next_seg <= mcb_seg {
        return mcb.size;
    }
    
    let next = MCB::read(machine, next_seg);
    if next.owner == 0 {
        mcb.size.wrapping_add(1).wrapping_add(next.size)
    } else {
        mcb.size
    }
}

/// Объединяет смежные свободные блоки в цепочке, начиная с first_seg.
fn coalesce(machine: &mut dyn Machine, first_seg: u16) {
    let mut seg = first_seg;
    loop {
        let mcb = MCB::read(machine, seg);
        if mcb.signature == MCB_SIGNATURE_LAST {
            break;
        }
        
        // ✅ ЗАЩИТА: безопасное сложение
        let next_seg = seg.wrapping_add(1).wrapping_add(mcb.size);
        
        // Если сегмент "переполнился" и стал меньше или равен текущему, цепочка повреждена
        if next_seg <= seg {
            log::warn!("MCB chain corrupted at segment {:04X}, breaking coalesce", seg);
            break;
        }
        
        let next = MCB::read(machine, next_seg);
        
        if mcb.owner == 0 && next.owner == 0 {
            // объединяем
            let new_size = mcb.size.wrapping_add(1).wrapping_add(next.size);
            MCB {
                size: new_size,
                signature: next.signature,
                ..mcb
            }
            .write(machine, seg);
        } else {
            seg = next_seg;
        }
    }
}