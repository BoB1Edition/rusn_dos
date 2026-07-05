// Ver: 2 File: ./libs/dos_core/src/instructions/segment.rs
use crate::{DosMachine, modrm::ModRm};

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
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    if modrm.is_register_mode() {
        log::error!(
            "LDS with register mode (mod=11) at CS:IP={:#04x}:{:#04x} — invalid opcode",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.halted = true;
        return;
    }
    let addr = match modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes) {
        Some(a) => a,
        None => {
            log::error!("LDS: Failed to resolve memory address");
            machine.halted = true;
            return;
        }
    };
    let offset = machine.read_phys_u16(addr);
    let segment = machine.read_phys_u16(addr.wrapping_add(2));
    machine.write_reg16(modrm.reg_field, offset);
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