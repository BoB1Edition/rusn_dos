use crate::{DosMachine, flags, modrm::ModRm};

fn perform_shift(op_field: u8, value: u32, count: u8, flags: u16) -> (u32, u16) {
    let count = count & 0x1F; // x86: count mod 32
    if count == 0 {
        return (value, flags);
    }

    let mut new_flags = flags;
    let af = false;
    let result = match op_field {
        0 => {
            // ROL
            let result = value.rotate_left(count as u32);
            let cf = (result >> 31) != 0;
            let msb_before = (value >> 31) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        1 => {
            // ROR
            let result = value.rotate_right(count as u32);
            let cf = (result & 1) != 0;
            let lsb_before = (value & 1) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                lsb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        2 => {
            // RCL - Rotate Left through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u64) | ((carry_in as u64) << 32);
            let rotated = input.rotate_left(count as u32);
            let result = rotated as u32;
            let new_cf = (rotated >> 32) != 0;
            let msb_before = (value >> 31) != 0;
            let msb_after = (result >> 31) != 0;
            let of = if count == 1 {
                msb_before != msb_after
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, new_cf, of, af);
            result
        }
        3 => {
            // RCR - Rotate Right through Carry
            let carry_in = (flags & 1) != 0;
            let input = (value as u64) | ((carry_in as u64) << 32);
            let rotated = input.rotate_right(count as u32);
            let result = rotated as u32;
            let new_cf = (rotated >> 32) != 0;
            let of = if count == 1 {
                let msb = (result >> 31) & 1;
                let second_msb = (result >> 30) & 1;
                msb != second_msb
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, new_cf, of, af);
            result
        }
        4 | 6 => {
            // SHL / SAL - Shift Left Arithmetic/Logical (одинаково)
            let extended = value as u64;
            let shifted = extended << count;
            let result = shifted as u32;
            let cf = (shifted >> 32) != 0;
            let of = if count == 1 {
                let msb_result = (result >> 31) & 1;
                let cf_bit = cf as u32;
                (msb_result ^ cf_bit) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        5 => {
            // SHR - Shift Right Logical
            let shifted = value >> count;
            let result = shifted;
            let cf = (value >> (count - 1)) & 1 != 0;
            let of = if count == 1 {
                (((result >> 31) ^ (cf as u32)) & 1) != 0
            } else {
                false
            };
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        7 => {
            // SAR - Shift Right Arithmetic
            let shifted = (value as i32).wrapping_shr(count as u32) as u32;
            let result = shifted;
            let cf = if count == 0 {
                false
            } else {
                (value >> (count - 1)) & 1 != 0
            };
            let of = false; // OF cleared for SAR
            new_flags = flags::compute_flags_u32(result, cf, of, af);
            result
        }
        _ => unreachable!(),
    };

    (result, new_flags)
}

pub fn shift_group_d1_32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None); // продвигаем только на байт ModR/M
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    if modrm.is_register_mode() {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR reg32, 1
        let value = machine.read_reg32(modrm.rm_field);
        let (result, new_flags) = perform_shift(modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_reg32(modrm.rm_field, result);
        machine.registers.set_flags(new_flags);
    } else {
        // ROL/ROR/RCL/RCR/SHL/SHR/SAR [mem], 1
        let addr = modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        let value = machine.read_phys_u32(addr);
        let (result, new_flags) = perform_shift(modrm.reg_field, value, 1, machine.registers.flags());
        machine.write_phys_u32(addr, result);
        machine.registers.set_flags(new_flags);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn shift_group_c1_32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    let imm8 = machine.read_u8(
        machine.registers.cs(),
        machine.registers.ip().wrapping_add(1),
    );
    machine.registers.step(Some(2));
    bytes.push(modrm_byte);
    bytes.push(imm8);
    let modrm = ModRm::from_byte(modrm_byte);
    
    let (value, addr_opt) = if modrm.is_register_mode() {
        (machine.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        bytes.extend_from_slice(&addr.to_le_bytes());
        (machine.read_phys_u32(addr), Some(addr))
    };
    
    let (result, new_flags) = perform_shift(modrm.reg_field, value, imm8, machine.registers.flags());
    
    if let Some(addr) = addr_opt {
        // Запись обратно в память
        machine.write_phys_u32(addr, result);
    } else {
        // Запись в регистр
        machine.write_reg32(modrm.rm_field, result);
    }
    
    machine.registers.set_flags(new_flags);
    machine.log_instruction(csip, &bytes).ok();
}