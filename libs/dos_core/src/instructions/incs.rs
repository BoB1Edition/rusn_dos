// Ver: 1 File: ./libs/dos_core/src/instructions/incs.rs

use crate::DosMachine;
use crate::flags;
use crate::inc_reg16;
use crate::dec_reg16;
use crate::modrm::ModRm;

inc_reg16!(inc_ax, ax, set_ax, 0x40);
inc_reg16!(inc_cx, cx, set_cx, 0x41);
inc_reg16!(inc_dx, dx, set_dx, 0x42);
inc_reg16!(inc_bx, bx, set_bx, 0x43);
inc_reg16!(inc_sp, sp, set_sp, 0x44);
inc_reg16!(inc_bp, bp, set_bp, 0x45);
inc_reg16!(inc_si, si, set_si, 0x46);
inc_reg16!(inc_di, di, set_di, 0x47);

dec_reg16!(dec_ax, ax, set_ax, 0x48);
dec_reg16!(dec_cx, cx, set_cx, 0x49);
dec_reg16!(dec_dx, dx, set_dx, 0x4A);
dec_reg16!(dec_bx, bx, set_bx, 0x4B);
dec_reg16!(dec_sp, sp, set_sp, 0x4C);
dec_reg16!(dec_bp, bp, set_bp, 0x4D);
dec_reg16!(dec_si, si, set_si, 0x4E);
dec_reg16!(dec_di, di, set_di, 0x4F);

pub(crate) fn inc_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Сохраняем текущее значение флага переноса (CF не изменяется!)
    let old_cf = machine.registers.flags() & 1;

    if modrm.is_register_mode() {
        // INC reg16
        let old_val = machine.read_reg16(modrm.rm_field);
        let new_val = old_val.wrapping_add(1);
        machine.write_reg16(modrm.rm_field, new_val);

        // Устанавливаем флаги (кроме CF)
        let cf = old_val == 0xFFFF; // перенос при 0xFFFF → 0x0000
        let of = old_val == 0x7FFF; // переполнение при 0x7FFF → 0x8000
        let af = (old_val & 0x0F) == 0x0F; // вспомогательный перенос

        let mut flags = flags::compute_flags_u16(machine.registers.flags(), new_val, cf, of, af);
        flags = (flags & !1) | old_cf; // восстанавливаем старый CF
        machine.registers.set_flags(flags);
    } else {
        // INC [mem]
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());

        let old_val = machine.read_phys_u16(addr);
        let new_val = old_val.wrapping_add(1);
        machine.write_phys_u16(addr, new_val);
        let cf = old_val == 0xFFFF;
        let of = old_val == 0x7FFF;
        let af = (old_val & 0x0F) == 0x0F;

        let mut flags = flags::compute_flags_u16(machine.registers.flags(), new_val, cf, of, af);
        flags = (flags & !1) | old_cf;
        machine.registers.set_flags(flags);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn dec_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let old_cf = machine.registers.flags() & 1;

    if modrm.is_register_mode() {
        let old_val = machine.read_reg16(modrm.rm_field);
        let new_val = old_val.wrapping_sub(1);
        machine.write_reg16(modrm.rm_field, new_val);

        let cf = old_val == 0;
        let of = old_val == 0x8000;
        let af = (old_val & 0x0F) == 0;

        let mut flags = flags::compute_flags_u16(machine.registers.flags(), new_val, cf, of, af);
        flags = (flags & !1) | old_cf;
        machine.registers.set_flags(flags);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());

        let old_val = machine.read_phys_u16(addr);
        let new_val = old_val.wrapping_sub(1);
        machine.write_phys_u16(addr, new_val);

        let cf = old_val == 0;
        let of = old_val == 0x8000;
        let af = (old_val & 0x0F) == 0;

        let mut flags = flags::compute_flags_u16(machine.registers.flags(), new_val, cf, of, af);
        flags = (flags & !1) | old_cf;
        machine.registers.set_flags(flags);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn inc_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let old_cf = machine.registers.flags() & 1;

    if modrm.is_register_mode() {
        let old_val = machine.read_reg32(modrm.rm_field);
        let new_val = old_val.wrapping_add(1);
        machine.write_reg32(modrm.rm_field, new_val);

        let cf = old_val == 0xFFFFFFFF;
        let of = old_val == 0x7FFFFFFF;
        let af = (old_val & 0x0F) == 0x0F;

        let mut flags = flags::compute_flags_u32(machine.registers.flags(), new_val, cf, of, af);
        flags = (flags & !1) | (old_cf);
        machine.registers.set_flags(flags as u16);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());

        let old_val = machine.read_phys_u32(addr);
        let new_val = old_val.wrapping_add(1);
        machine.write_phys_u32(addr, new_val);

        let cf = old_val == 0xFFFFFFFF;
        let of = old_val == 0x7FFFFFFF;
        let af = (old_val & 0x0F) == 0x0F;

        let mut flags = flags::compute_flags_u32(machine.registers.flags(), new_val, cf, of, af);
        flags = (flags & !1) | (old_cf);
        machine.registers.set_flags(flags as u16);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub fn dec_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let old_cf = machine.registers.flags() & 1;

    if modrm.is_register_mode() {
        let old_val = machine.read_reg32(modrm.rm_field);
        let new_val = old_val.wrapping_sub(1);
        machine.write_reg32(modrm.rm_field, new_val);

        let cf = old_val == 0;
        let of = old_val == 0x80000000;
        let af = (old_val & 0x0F) == 0;

        let mut flags = flags::compute_flags_u32(machine.registers.flags(), new_val, cf, of, af);
        flags = (flags & !1) | (old_cf);
        machine.registers.set_flags(flags as u16);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());

        let old_val = machine.read_phys_u32(addr);
        let new_val = old_val.wrapping_sub(1);
        machine.write_phys_u32(addr, new_val);

        let cf = old_val == 0;
        let of = old_val == 0x80000000;
        let af = (old_val & 0x0F) == 0;

        let mut flags = flags::compute_flags_u32(machine.registers.flags(), new_val, cf, of, af);
        flags = (flags & !1) | (old_cf);
        machine.registers.set_flags(flags as u16);
    }

    machine.log_instruction(csip, &bytes).ok();
}
