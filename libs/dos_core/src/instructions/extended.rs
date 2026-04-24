// Ver: 2
use crate::DosMachine;

pub fn movzx_r16_rm8(machine: &mut DosMachine, prev: &[u8]) { // ← ПЕРЕИМЕНОВАТЬ!
    let csip = [machine.registers.cs(), machine.registers.ip()  - prev.len() as u16];
    let modrm_byte = machine.read_instr_u8( machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = crate::modrm::ModRm::from_byte(modrm_byte);
    let dst_reg = modrm.reg_field;
    
    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field) as u16 
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u8(addr) as u16
    };
    
    machine.write_reg16(dst_reg, src_val);
    machine.log_instruction(csip, &bytes).ok();
}