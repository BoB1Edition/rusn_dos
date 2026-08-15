// Ver: 1 File: ./libs/dos_core/src/instructions/exchange.rs
use crate::{DosMachine, modrm::ModRm, xchg_ax_reg16, xchg_eax_reg32};

pub(crate) fn xchg_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    if modrm.is_register_mode() {
        // XCHG reg16, reg16
        let src_val = machine.read_reg16(modrm.reg_field);
        let dst_val = machine.read_reg16(modrm.rm_field);
        machine.write_reg16(modrm.rm_field, src_val);
        machine.write_reg16(modrm.reg_field, dst_val);
    } else {
        // XCHG [addr], reg16
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let mem_val = machine.read_phys_u16(addr);
        let reg_val = machine.read_reg16(modrm.reg_field);
        machine.write_phys_u16(addr, reg_val);
        machine.write_reg16(modrm.reg_field, mem_val);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn xchg_rm32_r32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    if modrm.is_register_mode() {
        let src_val = machine.read_reg32(modrm.reg_field);
        let dst_val = machine.read_reg32(modrm.rm_field);
        machine.write_reg32(modrm.rm_field, src_val);
        machine.write_reg32(modrm.reg_field, dst_val);
    } else {
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        let mem_val = machine.read_phys_u32(addr);
        let reg_val = machine.read_reg32(modrm.reg_field);
        machine.write_phys_u32(addr, reg_val);
        machine.write_reg32(modrm.reg_field, mem_val);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

xchg_ax_reg16!(xchg_ax_cx, cx, set_cx);
xchg_ax_reg16!(xchg_ax_dx, dx, set_dx);
xchg_ax_reg16!(xchg_ax_bx, bx, set_bx);
xchg_ax_reg16!(xchg_ax_sp, sp, set_sp);
xchg_ax_reg16!(xchg_ax_bp, bp, set_bp);
xchg_ax_reg16!(xchg_ax_si, si, set_si);
xchg_ax_reg16!(xchg_ax_di, di, set_di);

xchg_eax_reg32!(xchg_eax_ecx, ecx, set_ecx);
xchg_eax_reg32!(xchg_eax_edx, edx, set_edx);
xchg_eax_reg32!(xchg_eax_ebx, ebx, set_ebx);
xchg_eax_reg32!(xchg_eax_esp, esp, set_esp);
xchg_eax_reg32!(xchg_eax_ebp, ebp, set_ebp);
xchg_eax_reg32!(xchg_eax_esi, esi, set_esi);
xchg_eax_reg32!(xchg_eax_edi, edi, set_edi);

/// XCHG r/m8, r8 — опкод 0x86 /r
/// Обмен байтами между регистром и регистром/памятью
pub(crate) fn xchg_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Читаем значение из регистра (reg_field)
    let reg_val = machine.read_reg8(modrm.reg_field);

    if modrm.is_register_mode() {
        // Режим регистр-регистр
        let rm_val = machine.read_reg8(modrm.rm_field);
        machine.write_reg8(modrm.reg_field, rm_val);
        machine.write_reg8(modrm.rm_field, reg_val);
    } else {
        // Режим регистр-память
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        
        let mem_val = machine.read_phys_u8(addr);
        machine.write_reg8(modrm.reg_field, mem_val);
        machine.write_phys_u8(addr, reg_val);
    }

    machine.log_instruction(csip, &bytes).ok();
}