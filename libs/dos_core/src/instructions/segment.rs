
use crate::{DosMachine, modrm::ModRm};

/// LES r16, m16:16 — Load ES and register from far pointer
/// Читает 32-битный указатель из памяти в формате "смещение:сегмент" (little-endian)
/// и загружает младшие 16 бит в регистр, старшие 16 бит в сегментный регистр ES
pub fn les_r16_m16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
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
    
    // Вычисляем адрес источника (с учётом сегментных префиксов)
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
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