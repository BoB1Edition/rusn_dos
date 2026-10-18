// Ver: 2 File: ./crate/x86/src/instructions/group.rs
use crate::cpu::X86Cpu;
use crate::flags;
use crate::instructions::stack;
use crate::modrm::ModRm;
use bus::Machine;

pub fn group_f6(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let (value, addr) = if modrm.is_register_mode() {
        (cpu.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u8(addr), Some(addr))
    };
    match modrm.reg_field {
        0 | 1 => {
            let imm = cpu.fetch_u8(machine);
            let result = value & imm;
            cpu.registers
                .set_flags(flags::compute_logical_flags_u8(cpu.registers.flags(), result));
        }
        2 => write8(cpu, machine, modrm, addr, !value),
        3 => {
            let result = 0u8.wrapping_sub(value);
            let cf = value != 0;
            let of = value == 0x80;
            let af = (value & 0x0F) != 0;
            cpu.registers.set_flags(flags::compute_flags_u8(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
            write8(cpu, machine, modrm, addr, result);
        }
        4 => {
            let r = cpu.registers.al() as u16 * value as u16;
            cpu.registers.set_ax(r);
            mul_flags(cpu, r > 0xFF);
        }
        5 => {
            let r = (cpu.registers.al() as i8 as i16) * (value as i8 as i16);
            cpu.registers.set_ax(r as u16);
            mul_flags(cpu, r as i8 as i16 != r);
        }
        6 => {
            if value == 0 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            let ax = cpu.registers.ax();
            let quotient = ax / value as u16;
            if quotient > 0xFF {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            cpu.registers.set_al(quotient as u8);
            cpu.registers.set_ah((ax % value as u16) as u8);
        }
        7 => {
            if value == 0 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            let ax = cpu.registers.ax() as i16;
            let d = value as i8 as i16;
            let q = ax / d;
            let rem = ax % d;
            if q != q as i8 as i16 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            cpu.registers.set_al(q as u8);
            cpu.registers.set_ah(rem as u8);
        }
        _ => unreachable!(),
    }
}

pub fn group_f7(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        group_f7_32(cpu, machine, modrm);
    } else {
        group_f7_16(cpu, machine, modrm);
    }
}

fn group_f7_16(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) {
    let (value, addr) = if modrm.is_register_mode() {
        (cpu.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u16(addr), Some(addr))
    };
    match modrm.reg_field {
        0 | 1 => {
            let imm = cpu.fetch_u16(machine);
            let result = value & imm;
            cpu.registers
                .set_flags(flags::compute_logical_flags_u16(cpu.registers.flags(), result));
        }
        2 => write16(cpu, machine, modrm, addr, !value),
        3 => {
            let result = 0u16.wrapping_sub(value);
            let cf = value != 0;
            let of = value == 0x8000;
            let af = (value & 0x0F) != 0;
            cpu.registers.set_flags(flags::compute_flags_u16(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
            write16(cpu, machine, modrm, addr, result);
        }
        4 => {
            let r = cpu.registers.ax() as u32 * value as u32;
            cpu.registers.set_ax(r as u16);
            cpu.registers.set_dx((r >> 16) as u16);
            mul_flags(cpu, r > 0xFFFF);
        }
        5 => {
            let r = (cpu.registers.ax() as i16 as i32) * (value as i16 as i32);
            cpu.registers.set_ax(r as u16);
            cpu.registers.set_dx((r >> 16) as u16);
            mul_flags(cpu, r as i16 as i32 != r);
        }
        6 => {
            if value == 0 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            let dividend = ((cpu.registers.dx() as u32) << 16) | cpu.registers.ax() as u32;
            let q = dividend / value as u32;
            if q > 0xFFFF {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            cpu.registers.set_ax(q as u16);
            cpu.registers.set_dx((dividend % value as u32) as u16);
        }
        7 => {
            if value == 0 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            let dividend = (((cpu.registers.dx() as u32) << 16) | cpu.registers.ax() as u32) as i32;
            let d = value as i16 as i32;
            let q = dividend / d;
            if q != q as i16 as i32 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            cpu.registers.set_ax(q as u16);
            cpu.registers.set_dx((dividend % d) as u16);
        }
        _ => unreachable!(),
    }
}

fn group_f7_32(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) {
    let (value, addr) = if modrm.is_register_mode() {
        (cpu.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u32(addr), Some(addr))
    };
    match modrm.reg_field {
        0 | 1 => {
            let imm = cpu.fetch_u32(machine);
            let result = value & imm;
            cpu.registers
                .set_flags(flags::compute_logical_flags_u32(cpu.registers.flags(), result));
        }
        2 => write32(cpu, machine, modrm, addr, !value),
        3 => {
            let result = 0u32.wrapping_sub(value);
            let cf = value != 0;
            let of = value == 0x8000_0000;
            let af = (value & 0x0F) != 0;
            cpu.registers.set_flags(flags::compute_flags_u32(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
            write32(cpu, machine, modrm, addr, result);
        }
        4 => {
            let r = cpu.registers.eax() as u64 * value as u64;
            cpu.registers.set_eax(r as u32);
            cpu.registers.set_edx((r >> 32) as u32);
            mul_flags(cpu, r > 0xFFFF_FFFF);
        }
        5 => {
            let r = (cpu.registers.eax() as i32 as i64) * (value as i32 as i64);
            cpu.registers.set_eax(r as u32);
            cpu.registers.set_edx((r >> 32) as u32);
            mul_flags(cpu, r as i32 as i64 != r);
        }
        6 => {
            // DIV r/m32 (беззнаковое 64/32 → 32+32)
            if value == 0 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            let dividend = ((cpu.registers.edx() as u64) << 32) | cpu.registers.eax() as u64;
            let q = dividend / value as u64;
            if q > 0xFFFF_FFFF {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            cpu.registers.set_eax(q as u32);
            cpu.registers.set_edx((dividend % value as u64) as u32);
        }
        7 => {
            // IDIV r/m32 (знаковое 64/32 → 32+32)
            if value == 0 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            let dividend = (((cpu.registers.edx() as u64) << 32) | cpu.registers.eax() as u64) as i64;
            let d = value as i32 as i64;
            if dividend == i64::MIN && d == -1 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            let q = dividend / d;
            if q != q as i32 as i64 {
                crate::instructions::system::software_interrupt(cpu, machine, 0);
                return;
            }
            cpu.registers.set_eax(q as u32);
            cpu.registers.set_edx((dividend % d) as u32);
        }
        _ => unreachable!(),
    }
}

pub fn group_fe(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let (value, addr) = if modrm.is_register_mode() {
        (cpu.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u8(addr), Some(addr))
    };
    let old_cf = cpu.registers.flags() & flags::CF;
    match modrm.reg_field {
        0 => {
            let result = value.wrapping_add(1);
            let of = value == 0x7F;
            let af = (value & 0x0F) == 0x0F;
            let mut f = flags::compute_flags_u8(cpu.registers.flags(), result, false, of, af);
            f = (f & !flags::CF) | old_cf;
            cpu.registers.set_flags(f);
            write8(cpu, machine, modrm, addr, result);
        }
        1 => {
            let result = value.wrapping_sub(1);
            let of = value == 0x80;
            let af = (value & 0x0F) == 0;
            let mut f = flags::compute_flags_u8(cpu.registers.flags(), result, false, of, af);
            f = (f & !flags::CF) | old_cf;
            cpu.registers.set_flags(f);
            write8(cpu, machine, modrm, addr, result);
        }
        _ => {
            log::error!("FE /{} undefined", modrm.reg_field);
            cpu.halted = true;
        }
    }
}

pub fn group_ff(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    match modrm.reg_field {
        0 | 1 => incdec_wide(cpu, machine, modrm, modrm.reg_field == 0),
        2 => call_near(cpu, machine, modrm),
        3 => call_far(cpu, machine, modrm),
        4 => jmp_near(cpu, machine, modrm),
        5 => jmp_far(cpu, machine, modrm),
        6 => push_rm(cpu, machine, modrm),
        _ => {
            log::error!("FF /7 undefined");
            cpu.halted = true;
        }
    }
}

fn incdec_wide(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, inc: bool) {
    let old_cf = cpu.registers.flags() & flags::CF;
    if cpu.prefixes.has_operand_size {
        let (value, addr) = read32(cpu, machine, modrm);
        let result = if inc { value.wrapping_add(1) } else { value.wrapping_sub(1) };
        let of = if inc { value == 0x7FFF_FFFF } else { value == 0x8000_0000 };
        let af = if inc { (value & 0x0F) == 0x0F } else { (value & 0x0F) == 0 };
        let mut f = flags::compute_flags_u32(cpu.registers.flags(), result, false, of, af);
        f = (f & !flags::CF) | old_cf;
        cpu.registers.set_flags(f);
        write32(cpu, machine, modrm, addr, result);
    } else {
        let (value, addr) = read16(cpu, machine, modrm);
        let result = if inc { value.wrapping_add(1) } else { value.wrapping_sub(1) };
        let of = if inc { value == 0x7FFF } else { value == 0x8000 };
        let af = if inc { (value & 0x0F) == 0x0F } else { (value & 0x0F) == 0 };
        let mut f = flags::compute_flags_u16(cpu.registers.flags(), result, false, of, af);
        f = (f & !flags::CF) | old_cf;
        cpu.registers.set_flags(f);
        write16(cpu, machine, modrm, addr, result);
    }
}

fn call_near(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) {
    if cpu.prefixes.has_operand_size {
        let (target, _) = read32(cpu, machine, modrm);
        let ret = cpu.registers.eip();
        stack::push32(cpu, machine, ret);
        cpu.registers.set_eip(target);
    } else {
        let (target, _) = read16(cpu, machine, modrm);
        let ret = cpu.registers.ip();
        stack::push16(cpu, machine, ret);
        cpu.registers.set_ip(target);
    }
}

fn call_far(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) {
    if modrm.is_register_mode() {
        cpu.halted = true;
        return;
    }
    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
    let ip = machine.read_mem_u16(addr);
    let cs = machine.read_mem_u16(addr.wrapping_add(2));
    let ret_cs = cpu.registers.cs();
    let ret_ip = cpu.registers.ip();
    stack::push16(cpu, machine, ret_cs);
    stack::push16(cpu, machine, ret_ip);
    cpu.registers.set_cs(cs);
    cpu.registers.set_ip(ip);
}

fn jmp_near(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) {
    if cpu.prefixes.has_operand_size {
        let (target, _) = read32(cpu, machine, modrm);
        cpu.registers.set_eip(target);
    } else {
        let (target, _) = read16(cpu, machine, modrm);
        cpu.registers.set_ip(target);
    }
}

fn jmp_far(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) {
    if modrm.is_register_mode() {
        cpu.halted = true;
        return;
    }
    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
    let ip = machine.read_mem_u16(addr);
    let cs = machine.read_mem_u16(addr.wrapping_add(2));
    cpu.registers.set_cs(cs);
    cpu.registers.set_ip(ip);
}

fn push_rm(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) {
    if cpu.prefixes.has_operand_size {
        let (value, _) = read32(cpu, machine, modrm);
        stack::push32(cpu, machine, value);
    } else {
        let (value, _) = read16(cpu, machine, modrm);
        stack::push16(cpu, machine, value);
    }
}

fn mul_flags(cpu: &mut X86Cpu, overflow: bool) {
    let mut f = cpu.registers.flags();
    if overflow {
        f |= flags::CF | flags::OF;
    } else {
        f &= !(flags::CF | flags::OF);
    }
    cpu.registers.set_flags(f);
}

fn write8(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, v: u8) {
    if let Some(addr) = addr {
        machine.write_mem_u8(addr, v);
    } else {
        cpu.write_reg8(modrm.rm_field, v);
    }
}
fn write16(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, v: u16) {
    if let Some(addr) = addr {
        machine.write_mem_u16(addr, v);
    } else {
        cpu.write_reg16(modrm.rm_field, v);
    }
}
fn write32(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, v: u32) {
    if let Some(addr) = addr {
        machine.write_mem_u32(addr, v);
    } else {
        cpu.write_reg32(modrm.rm_field, v);
    }
}
fn read16(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) -> (u16, Option<u32>) {
    if modrm.is_register_mode() {
        (cpu.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u16(addr), Some(addr))
    }
}
fn read32(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) -> (u32, Option<u32>) {
    if modrm.is_register_mode() {
        (cpu.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u32(addr), Some(addr))
    }
}
