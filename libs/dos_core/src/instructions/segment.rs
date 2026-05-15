
use crate::{DosMachine, modrm::ModRm};

/// LES r16, m16:16 — Load ES and register from far pointer
/// Читает 32-битный указатель из памяти в формате "смещение:сегмент" (little-endian)
/// и загружает младшие 16 бит в регистр, старшие 16 бит в сегментный регистр ES
pub(crate) fn les_r16_m16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    // LES с регистровым режимом (mod=11) недопустим — вызывает #UD
    if modrm.is_register_mode() {
        log::error!(
            "LES with register mode (mod=11) is undefined behavior at {:#04x}:{:#04x}",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.halted = true;
        return;
    }
    
    let addr = modrm
        .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
        .unwrap();
    
    // Читаем 32 бита из памяти (little-endian: смещение, затем сегмент)
    let offset = machine.read_phys_u16(addr);
    let segment = machine.read_phys_u16(addr + 2);
    
    // Загружаем значения в регистры
    machine.write_reg16(modrm.reg_field, offset);
    machine.registers.set_es(segment);
    
    // Флаги НЕ изменяются
    machine.log_instruction(csip, &bytes).ok();
}

pub fn lds_r16_m16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем байт ModR/M
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    
    let modrm = ModRm::from_byte(modrm_byte);
    
    // LDS не поддерживает режим регистра (mod=11) — это #UD
    if modrm.is_register_mode() {
        log::error!(
            "LDS with register mode (mod=11) at CS:IP={:#04x}:{:#04x} — invalid opcode",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.halted = true;
        return;
    }
    
    // Вычисляем адрес операнда памяти
    let addr = match modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes) {
        Some(a) => a,
        None => {
            log::error!("LDS: Failed to resolve memory address");
            machine.halted = true;
            return;
        }
    };
    
    // Читаем 32-битный указатель из памяти: [offset:segment]
    let offset = machine.read_phys_u16(addr);
    let segment = machine.read_phys_u16(addr.wrapping_add(2));
    
    // Загружаем регистр (из поля reg_field ModR/M)
    machine.write_reg16(modrm.reg_field, offset);
    
    // Загружаем сегмент DS
    machine.registers.set_ds(segment);
    
    log::debug!(
        "LDS: reg={} ← {:#04x}, DS ← {:#04x} (from phys addr {:#06x})",
        modrm.reg_field,
        offset,
        segment,
        addr
    );
    
    machine.log_instruction(csip, &bytes).ok();
}