// Ver: 1 File: ./libs/dos_core/src/instructions/mov.rs
use crate::{flags, machine::DosMachine, modrm::ModRm, mov_reg8_imm8};

pub(crate) fn mov_ax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let imm = machine.read_instr_u16(machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    machine.log_instruction(csip, &bytes).ok();
    machine.registers.set_ax(imm);
    machine.registers.step(Some(2));
}

pub(crate) fn mov_dx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let imm = machine.read_instr_u16(machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.set_dx(imm);
    machine.registers.step(Some(2));
}

pub(crate) fn mov_bx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let imm = machine.read_instr_u16(machine.registers.ip());
    bytes.extend_from_slice(&imm.to_le_bytes());
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.set_bx(imm);
    machine.registers.step(Some(2));
}

pub(crate) fn mov_rm16_sreg(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let sreg_value = match modrm.reg_field {
        0 => machine.registers.es(),
        1 => machine.registers.cs(),
        2 => machine.registers.ss(),
        3 => machine.registers.ds(),
        4 => machine.registers.fs(),
        5 => machine.registers.gs(),
        _ => unreachable!(),
    };
    if modrm.is_register_mode() {
        machine.write_reg16(modrm.rm_field, sreg_value);
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u16(phys_addr, sreg_value);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_rm16_r16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = machine.read_reg16(modrm.reg_field);
    if modrm.is_register_mode() {
        machine.write_reg16(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u16(phys_addr, src_val);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        //let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        //let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u16(phys_addr)
    };
    machine.write_reg16(modrm.reg_field, src_val);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_al_address16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let addr16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&addr16.to_le_bytes());
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let value = machine.read_u8(segment, addr16);
    let phys_addr = ((segment as u32) << 4).wrapping_add(addr16 as u32);
    machine.registers.set_al(value);
    log::trace!(
        "MOV AL, [addr]: segment={:#04x} (override={:?}), offset={:#04x}, phys={:#06x}",
        segment,
        machine.override_segment,
        addr16,
        phys_addr
    );
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_al_address32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let addr32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&addr32.to_le_bytes());
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr32 & 0xFFFF) as u16;
    let value = machine.read_u8(segment, offset);
    machine.registers.set_al(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_sreg_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u16(phys_addr)
    };
    match modrm.reg_field {
        0 => machine.registers.set_es(src_val),
        1 => {
            log::error!("Attempt to write to CS register");
            machine.halted = true;
            return;
        }
        2 => {
            machine.inhibit_interrupts = true;
            machine.registers.set_ss(src_val)
        },
        3 => machine.registers.set_ds(src_val),
        4 => machine.registers.set_fs(src_val),
        5 => {
            machine.registers.set_gs(src_val);
        }
        _ => {
            log::error!(
                "Invalid segment register field in MOV sreg, r/m16 {}",
                modrm.reg_field
            );
            machine.halted = true;
            return;
        }
    }

    machine.log_instruction(csip, &bytes).ok();
}


pub(crate) fn mov_r8_rm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let src_val = if modrm.is_register_mode() {
        machine.read_reg8(modrm.rm_field)
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        //let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        //let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u8(phys_addr)
    };
    machine.write_reg8(modrm.reg_field, src_val);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_rm16_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.reg_field != 0 {
        log::error!("Invalid reg_field {} for opcode 0xC7", modrm.reg_field);
        machine.halted = true;
        return;
    }

    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&imm16.to_le_bytes());

    if modrm.is_register_mode() {
        machine.write_reg16(modrm.rm_field, imm16);
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u16(phys_addr, imm16);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_address_ax(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();

    let segment = machine.override_segment.unwrap_or(machine.registers.ds());

    let offset = if machine.has_address_size_prefix {
        let addr32 = machine.read_instr_u32(machine.registers.ip());
        machine.registers.step(Some(4));
        bytes.extend_from_slice(&addr32.to_le_bytes());
        (addr32 & 0xFFFF) as u16
    } else {
        let addr16 = machine.read_instr_u16(machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&addr16.to_le_bytes());
        addr16
    };

    machine.write_u16(segment, offset, machine.registers.ax());

    let phys = ((segment as u32) << 4).wrapping_add(offset as u32);
    log::trace!(
        "MOV [addr], AX: seg={:#04x}, offset={:#04x}, phys={:#06x}, a20={}",
        segment,
        offset,
        phys,
        machine.a20_enabled
    );

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn stosw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    let ax = machine.registers.ax();
    machine.write_u16(machine.registers.es(), machine.registers.di(), ax);
    if df {
        machine
            .registers
            .set_di(machine.registers.di().wrapping_sub(2));
    } else {
        machine
            .registers
            .set_di(machine.registers.di().wrapping_add(2));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_si_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    machine.registers.set_si(imm16);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_di_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    machine.registers.set_di(imm16);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_cx_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    machine.registers.set_cx(imm16);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_ax_address16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let addr = if machine.has_address_size_prefix {
        let addr32 = machine.read_instr_u32(machine.registers.ip());
        machine.registers.step(Some(4));
        bytes.extend_from_slice(&addr32.to_le_bytes());
        (addr32 & 0xFFFF) as u16
    } else {
        let addr16 = machine.read_instr_u16(machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&addr16.to_le_bytes());
        addr16
    };
    let value = machine.read_u16(segment, addr);
    machine.registers.set_ax(value);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn stosb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    let al = machine.registers.al();
    machine.write_u8(machine.registers.es(), machine.registers.di(), al);
    if df {
        machine
            .registers
            .set_di(machine.registers.di().wrapping_sub(1));
    } else {
        machine
            .registers
            .set_di(machine.registers.di().wrapping_add(1));
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_rm8_r8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let src_val = machine.read_reg8(modrm.reg_field);
    if modrm.is_register_mode() {
        machine.write_reg8(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u8(phys_addr, src_val);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn lodsb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    let si = machine.registers.si();
    let al = machine.read_u8(segment, si);
    machine.registers.set_al(al);
    if df {
        machine.registers.set_si(si.wrapping_sub(1));
    } else {
        machine.registers.set_si(si.wrapping_add(1));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_rm8_imm8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();

    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.reg_field != 0 {
        log::error!(
            "Invalid reg_field {} in MOV r/m8, imm8 (opcode 0xC6). Only /0 is valid.",
            modrm.reg_field
        );
        machine.halted = true;
        return;
    }

    let imm8 = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    bytes.push(imm8);

    if modrm.is_register_mode() {
        machine.write_reg8(modrm.rm_field, imm8);
    } else {
        let phys_addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u8(phys_addr, imm8);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn lea_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    if modrm.is_register_mode() {
        log::warn!(
            "LEA with register mode (mod=11) at {:#04x}:{:#04x} — emulating as MOV",
            machine.registers.cs(),
            machine.registers.ip()
        );
        let src_val = machine.read_reg16(modrm.rm_field);
        machine.write_reg16(modrm.reg_field, src_val);
    } else {
        let offset = compute_lea_offset_16(machine, &modrm, &mut bytes);
        machine.write_reg16(modrm.reg_field, offset);
    }
    machine.log_instruction(csip, &bytes).ok();
}

fn compute_lea_offset_16(machine: &mut DosMachine, modrm: &ModRm, bytes: &mut Vec<u8>) -> u16 {
    let mod_field = modrm.mod_field; // ← уже 0-3
    let rm_field = modrm.rm_field; // ← уже 0-7

    let base = match (mod_field, rm_field) {
        (0, 0) => machine.registers.bx() as i32 + machine.registers.si() as i32,
        (0, 1) => machine.registers.bx() as i32 + machine.registers.di() as i32,
        (0, 2) => machine.registers.bp() as i32 + machine.registers.si() as i32,
        (0, 3) => machine.registers.bp() as i32 + machine.registers.di() as i32,
        (0, 4) => machine.registers.si() as i32,
        (0, 5) => machine.registers.di() as i32,
        (0, 6) => {
            let disp16 = machine.read_instr_u16(machine.registers.ip()) as i32;
            machine.registers.step(Some(2));
            bytes.extend_from_slice(&disp16.to_le_bytes());
            disp16
        }
        (0, 7) => machine.registers.bx() as i32,
        (1, 0) => machine.registers.bx() as i32 + machine.registers.si() as i32,
        (1, 1) => machine.registers.bx() as i32 + machine.registers.di() as i32,
        (1, 2) => machine.registers.bp() as i32 + machine.registers.si() as i32,
        (1, 3) => machine.registers.bp() as i32 + machine.registers.di() as i32,
        (1, 4) => machine.registers.si() as i32,
        (1, 5) => machine.registers.di() as i32,
        (1, 6) => machine.registers.bp() as i32,
        (1, 7) => machine.registers.bx() as i32,
        (2, 0) => machine.registers.bx() as i32 + machine.registers.si() as i32,
        (2, 1) => machine.registers.bx() as i32 + machine.registers.di() as i32,
        (2, 2) => machine.registers.bp() as i32 + machine.registers.si() as i32,
        (2, 3) => machine.registers.bp() as i32 + machine.registers.di() as i32,
        (2, 4) => machine.registers.si() as i32,
        (2, 5) => machine.registers.di() as i32,
        (2, 6) => machine.registers.bp() as i32,
        (2, 7) => machine.registers.bx() as i32,
        _ => unreachable!(),
    };

    let displacement = match mod_field {
        0 => 0,
        1 => {
            let disp8 = machine.read_instr_u8(machine.registers.ip()) as i8 as i32;
            machine.registers.step(None);
            bytes.push(disp8 as u8);
            disp8
        }
        2 => {
            // mod=10: disp16
            let disp16 = machine.read_instr_u16(machine.registers.ip()) as i32;
            machine.registers.step(Some(2));
            bytes.extend_from_slice(&disp16.to_le_bytes());
            disp16
        }
        _ => 0,
    };
    ((base + displacement) & 0xFFFF) as u16
}

pub(crate) fn cmpsb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    bytes.push(0xA6);

    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());

    let si = machine.registers.si();
    let di = machine.registers.di();
    let src_byte = machine.read_u8(src_segment, si);
    let dst_byte = machine.read_u8(machine.registers.es(), di);

    let result = src_byte.wrapping_sub(dst_byte);

    let cf = src_byte < dst_byte;

    let src_sign = (src_byte as i8) < 0;
    let dst_sign = (dst_byte as i8) < 0;
    let result_sign = (result as i8) < 0;
    let of = (src_sign != dst_sign) && (src_sign != result_sign);

    let af = (src_byte & 0x0F) < (dst_byte & 0x0F);

    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(1));
        machine.registers.set_di(di.wrapping_sub(1));
    } else {
        machine.registers.set_si(si.wrapping_add(1));
        machine.registers.set_di(di.wrapping_add(1));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn cmpsw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    bytes.push(0xA7); // опкод CMPSW
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let si = machine.registers.si();
    let di = machine.registers.di();
    let src_word = machine.read_u16(src_segment, si);
    let dst_word = machine.read_u16(machine.registers.es(), di);
    let result = src_word.wrapping_sub(dst_word);
    let cf = src_word < dst_word;
    let src_sign = (src_word as i16) < 0;
    let dst_sign = (dst_word as i16) < 0;
    let result_sign = (result as i16) < 0;
    let of = (src_sign != dst_sign) && (src_sign != result_sign);
    let af = (src_word & 0x0F) < (dst_word & 0x0F);
    machine.registers.set_flags(flags::compute_flags_u16(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(2));
        machine.registers.set_di(di.wrapping_sub(2));
    } else {
        machine.registers.set_si(si.wrapping_add(2));
        machine.registers.set_di(di.wrapping_add(2));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn movsb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    bytes.push(0xA4);
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let si = machine.registers.si();
    let byte = machine.read_u8(src_segment, si);
    let di = machine.registers.di();
    machine.write_u8(machine.registers.es(), di, byte);
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(1));
        machine.registers.set_di(di.wrapping_sub(1));
    } else {
        machine.registers.set_si(si.wrapping_add(1));
        machine.registers.set_di(di.wrapping_add(1));
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_bp_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    machine.registers.set_bp(imm16);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_ax_address32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let addr32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&addr32.to_le_bytes());
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let offset = (addr32 & 0xFFFF) as u16;
    let value = machine.read_u16(segment, offset);
    machine.registers.set_ax(value);

    let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
    log::trace!(
        "MOV AX, [addr32]: segment={:#04x}, offset32={:#08x}, offset16={:#04x}, phys={:#06x}, value={:#04x}",
        segment,
        addr32,
        offset,
        phys_addr,
        value
    );

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_address_al(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let addr = if machine.has_address_size_prefix {
        let addr32 = machine.read_instr_u32(machine.registers.ip());
        machine.registers.step(Some(4));
        bytes.extend_from_slice(&addr32.to_le_bytes());
        (addr32 & 0xFFFF) as u16
    } else {
        let addr16 = machine.read_instr_u16(machine.registers.ip());
        machine.registers.step(Some(2));
        bytes.extend_from_slice(&addr16.to_le_bytes());
        addr16
    };
    let segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let al = machine.registers.al();
    machine.write_u8(segment, addr, al);

    machine.log_instruction(csip, &bytes).ok();
}

/// SCASB — Compare AL with byte at [ES:DI], then update DI and flags
pub(crate) fn scasb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();
    let al = machine.registers.al();
    let di = machine.registers.di();
    let src_byte = machine.read_u8(machine.registers.es(), di);

    let result = al.wrapping_sub(src_byte);
    let cf = al < src_byte;
    let al_sign = (al as i8) < 0;
    let src_sign = (src_byte as i8) < 0;
    let result_sign = (result as i8) < 0;
    let of = (al_sign != src_sign) && (al_sign != result_sign);
    let af = (al & 0x0F) < (src_byte & 0x0F);

    machine.registers.set_flags(flags::compute_flags_u8(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_di(di.wrapping_sub(1));
    } else {
        machine.registers.set_di(di.wrapping_add(1));
    }
    machine.log_instruction(csip, &bytes).ok();
}

/// SCASW — Compare AX with word at [ES:DI], then update DI and flags
pub(crate) fn scasw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();
    let ax = machine.registers.ax();
    let di = machine.registers.di();
    let src_word = machine.read_u16(machine.registers.es(), di);

    let result = ax.wrapping_sub(src_word);
    let cf = ax < src_word;
    let ax_sign = (ax as i16) < 0;
    let src_sign = (src_word as i16) < 0;
    let result_sign = (result as i16) < 0;
    let of = (ax_sign != src_sign) && (ax_sign != result_sign);
    let af = (ax & 0x0F) < (src_word & 0x0F);

    machine.registers.set_flags(flags::compute_flags_u16(
        machine.registers.flags(),
        result,
        cf,
        of,
        af,
    ));

    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_di(di.wrapping_sub(2));
    } else {
        machine.registers.set_di(di.wrapping_add(2));
    }
    machine.log_instruction(csip, &bytes).ok();
}


mov_reg8_imm8!(mov_al_imm8, set_al, 0xB0);
mov_reg8_imm8!(mov_cl_imm8, set_cl, 0xB1);
mov_reg8_imm8!(mov_dl_imm8, set_dl, 0xB2);
mov_reg8_imm8!(mov_bl_imm8, set_bl, 0xB3);
mov_reg8_imm8!(mov_ah_imm8, set_ah, 0xB4);
mov_reg8_imm8!(mov_ch_imm8, set_ch, 0xB5);
mov_reg8_imm8!(mov_dh_imm8, set_dh, 0xB6);
mov_reg8_imm8!(mov_bh_imm8, set_bh, 0xB7);

/// MOVSW — Move String Word (DS:SI -> ES:DI, ±2)
pub(crate) fn movsw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let bytes = prev.to_vec(); // байты не пополняем, т.к. опкод уже в full_bytes
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    let si = machine.registers.si();
    let word = machine.read_u16(src_segment, si);
    let di = machine.registers.di();
    machine.write_u16(machine.registers.es(), di, word);
    let df = (machine.registers.flags() & (flags::DF)) != 0;
    if df {
        machine.registers.set_si(si.wrapping_sub(2));
        machine.registers.set_di(di.wrapping_sub(2));
    } else {
        machine.registers.set_si(si.wrapping_add(2));
        machine.registers.set_di(di.wrapping_add(2));
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn mov_sp_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&imm16.to_le_bytes());
    machine.registers.set_sp(imm16);
    machine.log_instruction(csip, &bytes).ok();
}