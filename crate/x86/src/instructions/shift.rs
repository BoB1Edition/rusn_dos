// Ver: 1 File: ./crate/x86/src/instructions/shift.rs
use crate::cpu::X86Cpu;
use crate::flags;
use crate::modrm::ModRm;
use bus::Machine;

pub fn shift8(cpu: &mut X86Cpu, machine: &mut dyn Machine, count_src: Count) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let count = count_of(cpu, machine, count_src);
    let (value, addr) = if modrm.is_register_mode() {
        (cpu.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u8(addr), Some(addr))
    };
    let (result, new_flags) = rotate8(modrm.reg_field, value, count, cpu.registers.flags());
    if let Some(addr) = addr {
        machine.write_mem_u8(addr, result);
    } else {
        cpu.write_reg8(modrm.rm_field, result);
    }
    cpu.registers.set_flags(new_flags);
}

pub fn shift_wide(cpu: &mut X86Cpu, machine: &mut dyn Machine, count_src: Count) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let count = count_of(cpu, machine, count_src);
    if cpu.prefixes.has_operand_size {
        let (value, addr) = if modrm.is_register_mode() {
            (cpu.read_reg32(modrm.rm_field), None)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            (machine.read_mem_u32(addr), Some(addr))
        };
        let (result, new_flags) = rotate32(modrm.reg_field, value, count, cpu.registers.flags());
        if let Some(addr) = addr {
            machine.write_mem_u32(addr, result);
        } else {
            cpu.write_reg32(modrm.rm_field, result);
        }
        cpu.registers.set_flags(new_flags);
    } else {
        let (value, addr) = if modrm.is_register_mode() {
            (cpu.read_reg16(modrm.rm_field), None)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            (machine.read_mem_u16(addr), Some(addr))
        };
        let (result, new_flags) = rotate16(modrm.reg_field, value, count, cpu.registers.flags());
        if let Some(addr) = addr {
            machine.write_mem_u16(addr, result);
        } else {
            cpu.write_reg16(modrm.rm_field, result);
        }
        cpu.registers.set_flags(new_flags);
    }
}

#[derive(Clone, Copy)]
pub enum Count {
    One,
    Cl,
    Imm8,
}

fn count_of(cpu: &mut X86Cpu, machine: &mut dyn Machine, src: Count) -> u8 {
    match src {
        Count::One => 1,
        Count::Cl => cpu.registers.cl(),
        Count::Imm8 => cpu.fetch_u8(machine),
    }
}

fn rotate8(op: u8, value: u8, count: u8, flags_in: u16) -> (u8, u16) {
    let count = count & 0x1F;
    if count == 0 {
        return (value, flags_in);
    }
    let cf_in = flags::test_cf(flags_in);
    let (result, cf, of) = match op {
        0 => rol(value as u32, count, 8),
        1 => ror(value as u32, count, 8),
        2 => rcl(value as u32, count, 8, cf_in),
        3 => rcr(value as u32, count, 8, cf_in),
        4 | 6 => shl(value as u32, count, 8),
        5 => shr(value as u32, count, 8),
        7 => sar(value as u32, count, 8),
        _ => (value as u32, false, false),
    };
    let result = result as u8;
    let out = flags::compute_flags_u8(flags_in, result, cf, of, false);
    (result, out)
}

fn rotate16(op: u8, value: u16, count: u8, flags_in: u16) -> (u16, u16) {
    let count = count & 0x1F;
    if count == 0 {
        return (value, flags_in);
    }
    let cf_in = flags::test_cf(flags_in);
    let (result, cf, of) = match op {
        0 => rol(value as u32, count, 16),
        1 => ror(value as u32, count, 16),
        2 => rcl(value as u32, count, 16, cf_in),
        3 => rcr(value as u32, count, 16, cf_in),
        4 | 6 => shl(value as u32, count, 16),
        5 => shr(value as u32, count, 16),
        7 => sar(value as u32, count, 16),
        _ => (value as u32, false, false),
    };
    let result = result as u16;
    (result, flags::compute_flags_u16(flags_in, result, cf, of, false))
}

fn rotate32(op: u8, value: u32, count: u8, flags_in: u16) -> (u32, u16) {
    let count = count & 0x1F;
    if count == 0 {
        return (value, flags_in);
    }
    let cf_in = flags::test_cf(flags_in);
    let (result, cf, of) = match op {
        0 => rol(value, count, 32),
        1 => ror(value, count, 32),
        2 => rcl(value, count, 32, cf_in),
        3 => rcr(value, count, 32, cf_in),
        4 | 6 => shl(value, count, 32),
        5 => shr(value, count, 32),
        7 => sar(value, count, 32),
        _ => (value, false, false),
    };
    (result, flags::compute_flags_u32(flags_in, result, cf, of, false))
}

fn rol(value: u32, count: u8, width: u32) -> (u32, bool, bool) {
    let mask = if width == 32 { u32::MAX } else { (1u32 << width) - 1 };
    let c = (count as u32) % width;
    let v = value & mask;
    let result = ((v << c) | (v >> (width - c))) & mask;
    let cf = (result & 1) != 0;
    let of = count == 1 && ((result >> (width - 1)) & 1) != (if cf { 1 } else { 0 });
    (result, cf, of)
}

fn ror(value: u32, count: u8, width: u32) -> (u32, bool, bool) {
    let mask = if width == 32 { u32::MAX } else { (1u32 << width) - 1 };
    let c = (count as u32) % width;
    let v = value & mask;
    let result = ((v >> c) | (v << (width - c))) & mask;
    let cf = ((result >> (width - 1)) & 1) != 0;
    let of = count == 1 && ((result >> (width - 1)) & 1) != ((result >> (width - 2)) & 1);
    (result, cf, of)
}

fn rcl(value: u32, count: u8, width: u32, cf_in: bool) -> (u32, bool, bool) {
    let modu = width + 1;
    let c = (count as u32) % modu;
    if c == 0 {
        return (value, cf_in, false);
    }
    let wide = if width == 32 { 64 } else { width + 1 };
    let mut bits = value as u64;
    if cf_in {
        bits |= 1u64 << width;
    }
    let mask = (1u64 << wide) - 1;
    let rotated = ((bits << c) | (bits >> (wide - c))) & mask;
    let result = (rotated & ((1u64 << width) - 1)) as u32;
    let cf = ((rotated >> width) & 1) != 0;
    let of = count == 1 && ((result >> (width - 1)) & 1) != (if cf { 1 } else { 0 });
    (result, cf, of)
}

fn rcr(value: u32, count: u8, width: u32, cf_in: bool) -> (u32, bool, bool) {
    let modu = width + 1;
    let c = (count as u32) % modu;
    if c == 0 {
        return (value, cf_in, false);
    }
    let old_sign = (value >> (width - 1)) & 1;
    let wide = if width == 32 { 64 } else { width + 1 };
    let mut bits = value as u64;
    if cf_in {
        bits |= 1u64 << width;
    }
    let mask = (1u64 << wide) - 1;
    let rotated = ((bits >> c) | (bits << (wide - c))) & mask;
    let result = (rotated & ((1u64 << width) - 1)) as u32;
    let cf = ((rotated >> width) & 1) != 0;
    let of = count == 1 && old_sign != ((result >> (width - 1)) & 1);
    (result, cf, of)
}

fn shl(value: u32, count: u8, width: u32) -> (u32, bool, bool) {
    let mask = if width == 32 { u32::MAX } else { (1u32 << width) - 1 };
    let v = value & mask;
    let c = count as u32;
    if c > width {
        return (0, false, false);
    }
    if c == width {
        return (0, (v & 1) != 0, false);
    }
    let shifted = (v as u64) << c;
    let result = (shifted as u32) & mask;
    let cf = ((shifted >> width) & 1) != 0;
    let of = count == 1 && ((v >> (width - 1)) & 1) != ((result >> (width - 1)) & 1);
    (result, cf, of)
}

fn shr(value: u32, count: u8, width: u32) -> (u32, bool, bool) {
    let mask = if width == 32 { u32::MAX } else { (1u32 << width) - 1 };
    let v = value & mask;
    let c = count as u32;
    if c >= width {
        let cf = c == width && ((v >> (width - 1)) & 1) != 0;
        return (0, cf, false);
    }
    let cf = ((v >> (c - 1)) & 1) != 0;
    let result = (v >> c) & mask;
    let of = count == 1 && ((v >> (width - 1)) & 1) != 0;
    (result, cf, of)
}

fn sar(value: u32, count: u8, width: u32) -> (u32, bool, bool) {
    let mask = if width == 32 { u32::MAX } else { (1u32 << width) - 1 };
    let v = value & mask;
    let sign = (v >> (width - 1)) & 1;
    let c = count as u32;
    if c >= width {
        let result = if sign != 0 { mask } else { 0 };
        return (result, sign != 0, false);
    }
    let cf = ((v >> (c - 1)) & 1) != 0;
    let mut result = v >> c;
    if sign != 0 {
        result |= mask << (width - c);
        result &= mask;
    }
    let of = false;
    (result, cf, of)
}
