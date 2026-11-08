// Ver: 3 File: crate/dos/src/mcb.rs
//! DOS Memory Control Block (MCB) allocator.
//! Real-mode paragraph-based memory manager.
use crate::consts::{MCB_SIGNATURE_LAST, MCB_SIGNATURE_NON_LAST};
use bus::Machine;

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
    let available = crate::consts::MAX_MCB_SEGMENT.saturating_sub(first_mcb_segment);
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

#[inline]
fn valid_mcb_signature(sig: u8) -> bool {
    sig == MCB_SIGNATURE_LAST || sig == MCB_SIGNATURE_NON_LAST
}

pub fn allocate(machine: &mut dyn Machine, first_seg: u16, paragraphs: u16) -> Option<u16> {
    let mut seg = first_seg;
    loop {
        if seg >= crate::consts::MAX_MCB_SEGMENT {
            log::warn!("MCB allocate: walked past MAX_MCB_SEGMENT at {:04X}", seg);
            break;
        }
        let mcb = MCB::read(machine, seg);

        if !valid_mcb_signature(mcb.signature) {
            log::warn!(
                "MCB allocate: bad signature {:02X} at {:04X}",
                mcb.signature,
                seg
            );
            break;
        }

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
            log::warn!(
                "MCB chain corrupted during allocate at segment {:04X}, breaking",
                seg
            );
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
        let excess = mcb.size.wrapping_sub(new_paragraphs);

        if excess == 1 {
            log::trace!(
                "MCB modify: cannot free 1 paragraph at {:04X}, keeping old size",
                mcb_seg
            );
        }
        if excess > 1 {
            let next_seg = mcb_seg.wrapping_add(1).wrapping_add(new_paragraphs);
            let next_sig = if mcb.signature == MCB_SIGNATURE_LAST {
                MCB_SIGNATURE_LAST
            } else {
                MCB_SIGNATURE_NON_LAST
            };

            // Текущий блок УМЕНЬШАЕТСЯ и становится НЕ-последним,
            // потому что за ним теперь есть свободный остаток.
            MCB {
                signature: MCB_SIGNATURE_NON_LAST,
                owner: mcb.owner,
                size: new_paragraphs,
            }
            .write(machine, mcb_seg);

            // Свободный остаток наследует признак последнего
            MCB {
                signature: next_sig,
                owner: 0,
                size: excess.wrapping_sub(1),
            }
            .write(machine, next_seg);
        }

        coalesce(machine, first_seg);
        Ok(0)
    } else {
        if mcb.signature == MCB_SIGNATURE_LAST {
            return Err(mcb.size);
        }

        if mcb.signature != MCB_SIGNATURE_NON_LAST {
            log::warn!(
                "MCB modify grow: bad signature {:02X} at {:04X}",
                mcb.signature,
                mcb_seg
            );
            return Err(mcb.size);
        }

        let next_seg = mcb_seg.wrapping_add(1).wrapping_add(mcb.size);

        if next_seg <= mcb_seg || next_seg >= crate::consts::MAX_MCB_SEGMENT {
            return Err(mcb.size);
        }

        let next = MCB::read(machine, next_seg);

        if next.signature != MCB_SIGNATURE_LAST && next.signature != MCB_SIGNATURE_NON_LAST {
            log::warn!(
                "MCB modify grow: bad next signature {:02X} at {:04X}",
                next.signature,
                next_seg
            );
            return Err(mcb.size);
        }
        let combined = mcb.size.wrapping_add(1).wrapping_add(next.size);

        if next.owner == 0 && combined >= new_paragraphs {
            let rest = combined.wrapping_sub(new_paragraphs);
            let (final_size, new_sig) = if rest > 1 {
                let new_next = mcb_seg.wrapping_add(1).wrapping_add(new_paragraphs);
                MCB {
                    signature: next.signature,
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
            // возвращаем ошибку с максимальным доступным размером
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
        if seg >= crate::consts::MAX_MCB_SEGMENT {
            log::warn!(
                "MCB max_available: walked past MAX_MCB_SEGMENT at {:04X}",
                seg
            );
            break;
        }

        let mcb = MCB::read(machine, seg);

        if !valid_mcb_signature(mcb.signature) {
            log::warn!(
                "MCB max_available: bad signature {:02X} at {:04X}",
                mcb.signature,
                seg
            );
            break;
        }

        if mcb.owner == 0 && mcb.size > max {
            max = mcb.size;
        }

        if mcb.signature == MCB_SIGNATURE_LAST {
            break;
        }

        let next_seg = seg.wrapping_add(1).wrapping_add(mcb.size);

        if next_seg <= seg || next_seg >= crate::consts::MAX_MCB_SEGMENT {
            log::warn!(
                "MCB max_available: corrupted chain at segment {:04X}, breaking",
                seg
            );
            break;
        }

        seg = next_seg;
    }

    max
}

/// Возвращает максимальный размер, до которого можно увеличить блок
/// (включая следующий свободный).
pub fn max_available_at(machine: &dyn Machine, first_seg: u16, data_segment: u16) -> u16 {
    let _ = first_seg;

    if data_segment == 0 {
        return 0;
    }

    let mcb_seg = data_segment - 1;
    let mcb = MCB::read(machine, mcb_seg);

    if mcb.signature == MCB_SIGNATURE_LAST {
        return mcb.size;
    }

    if mcb.signature != MCB_SIGNATURE_NON_LAST {
        log::warn!(
            "MCB max_available_at: bad signature {:02X} at {:04X}",
            mcb.signature,
            mcb_seg
        );
        return mcb.size;
    }

    let next_seg = mcb_seg.wrapping_add(1).wrapping_add(mcb.size);

    if next_seg <= mcb_seg || next_seg >= crate::consts::MAX_MCB_SEGMENT {
        return mcb.size;
    }

    let next = MCB::read(machine, next_seg);

    if next.signature != MCB_SIGNATURE_LAST && next.signature != MCB_SIGNATURE_NON_LAST {
        log::warn!(
            "MCB max_available_at: bad next signature {:02X} at {:04X}",
            next.signature,
            next_seg
        );
        return mcb.size;
    }

    if next.owner == 0 {
        mcb.size.wrapping_add(1).wrapping_add(next.size)
    } else {
        mcb.size
    }
}

fn coalesce(machine: &mut dyn Machine, first_seg: u16) {
    let mut seg = first_seg;

    loop {
        if seg >= crate::consts::MAX_MCB_SEGMENT {
            log::warn!("MCB coalesce: walked past MAX_MCB_SEGMENT at {:04X}", seg);
            break;
        }

        let mcb = MCB::read(machine, seg);

        if !valid_mcb_signature(mcb.signature) {
            log::warn!(
                "MCB coalesce: bad signature {:02X} at {:04X}",
                mcb.signature,
                seg
            );
            break;
        }

        if mcb.signature == MCB_SIGNATURE_LAST {
            break;
        }

        let next_seg = seg.wrapping_add(1).wrapping_add(mcb.size);

        if next_seg <= seg || next_seg >= crate::consts::MAX_MCB_SEGMENT {
            log::warn!(
                "MCB coalesce: corrupted chain at segment {:04X}, breaking",
                seg
            );
            break;
        }

        let next = MCB::read(machine, next_seg);

        if !valid_mcb_signature(next.signature) {
            log::warn!(
                "MCB coalesce: bad next signature {:02X} at {:04X}",
                next.signature,
                next_seg
            );
            break;
        }

        if mcb.owner == 0 && next.owner == 0 {
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

pub fn dump_chain(machine: &dyn Machine, first_seg: u16) {
    log::debug!("=== MCB chain dump from {:04X} ===", first_seg);
    let mut seg = first_seg;
    let mut n = 0u16;

    loop {
        // Защита от выхода за пределы разумной памяти и от зацикливания
        if seg >= crate::consts::MAX_MCB_SEGMENT || n > 256 {
            log::debug!("  dump stop: seg={:04X}, n={}", seg, n);
            break;
        }

        let mcb = MCB::read(machine, seg);
        let state = if mcb.owner == 0 { "free" } else { "used" };
        let sig = match mcb.signature {
            MCB_SIGNATURE_LAST => 'Z',
            MCB_SIGNATURE_NON_LAST => 'M',
            other => other as char,
        };

        log::debug!(
            "  [{}] seg={:04X} sig={} owner={:04X} size={:04X} ({})",
            n,
            seg,
            sig,
            mcb.owner,
            mcb.size,
            state
        );

        if mcb.signature != MCB_SIGNATURE_LAST && mcb.signature != MCB_SIGNATURE_NON_LAST {
            log::debug!(
                "  dump stop: bad signature {:02X} at {:04X}",
                mcb.signature,
                seg
            );
            break;
        }

        if mcb.signature == MCB_SIGNATURE_LAST {
            break;
        }

        let next_seg = seg.wrapping_add(1).wrapping_add(mcb.size);
        if next_seg <= seg {
            log::debug!("  dump stop: corrupted chain at {:04X}", seg);
            break;
        }
        seg = next_seg;
        n += 1;
    }
    log::debug!("=== end MCB dump ===");
}
