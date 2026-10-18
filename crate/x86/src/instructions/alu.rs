// Ver: 3 File: ./crate/x86/src/instructions/alu.rs
use crate::cpu::X86Cpu;
use crate::flags;
use crate::modrm::ModRm;
use bus::Machine;

#[inline]
fn set_flags(cpu: &mut X86Cpu, flags: u16) {
    cpu.registers.set_flags(flags);
}

pub fn add8(dst: u8, src: u8) -> (u8, bool, bool, bool) {
    let res = dst as u16 + src as u16;
    let result = res as u8;
    let cf = res > 0xFF;
    let af = ((dst & 0x0F) + (src & 0x0F)) > 0x0F;
    let of = ((dst ^ src) & 0x80) == 0 && ((dst ^ result) & 0x80) != 0;
    (result, cf, of, af)
}

pub fn add16(dst: u16, src: u16) -> (u16, bool, bool, bool) {
    let res = dst as u32 + src as u32;
    let result = res as u16;
    let cf = res > 0xFFFF;
    let af = ((dst & 0x0F) + (src & 0x0F)) > 0x0F;
    let of = ((dst ^ src) & 0x8000) == 0 && ((dst ^ result) & 0x8000) != 0;
    (result, cf, of, af)
}

pub fn add32(dst: u32, src: u32) -> (u32, bool, bool, bool) {
    let res = dst as u64 + src as u64;
    let result = res as u32;
    let cf = res > 0xFFFF_FFFF;
    let af = ((dst & 0x0F) + (src & 0x0F)) > 0x0F;
    let of = ((dst ^ src) & 0x8000_0000) == 0 && ((dst ^ result) & 0x8000_0000) != 0;
    (result, cf, of, af)
}

pub fn adc8(dst: u8, src: u8, carry: bool) -> (u8, bool, bool, bool) {
    let c = if carry { 1u16 } else { 0 };
    let res = dst as u16 + src as u16 + c;
    let result = res as u8;
    let cf = res > 0xFF;
    let af = ((dst & 0x0F) as u16 + (src & 0x0F) as u16 + c) > 0x0F;
    let of = ((dst ^ src) & 0x80) == 0 && ((dst ^ result) & 0x80) != 0;
    (result, cf, of, af)
}

pub fn adc16(dst: u16, src: u16, carry: bool) -> (u16, bool, bool, bool) {
    let c = if carry { 1u32 } else { 0 };
    let res = dst as u32 + src as u32 + c;
    let result = res as u16;
    let cf = res > 0xFFFF;
    let af = ((dst & 0x0F) as u32 + (src & 0x0F) as u32 + c) > 0x0F;
    let of = ((dst ^ src) & 0x8000) == 0 && ((dst ^ result) & 0x8000) != 0;
    (result, cf, of, af)
}

pub fn adc32(dst: u32, src: u32, carry: bool) -> (u32, bool, bool, bool) {
    let c = if carry { 1u64 } else { 0 };
    let res = dst as u64 + src as u64 + c;
    let result = res as u32;
    let cf = res > 0xFFFF_FFFF;
    let af = ((dst & 0x0F) as u64 + (src & 0x0F) as u64 + c) > 0x0F;
    let of = ((dst ^ src) & 0x8000_0000) == 0 && ((dst ^ result) & 0x8000_0000) != 0;
    (result, cf, of, af)
}

pub fn sub8(dst: u8, src: u8) -> (u8, bool, bool, bool) {
    let result = dst.wrapping_sub(src);
    let cf = dst < src;
    let af = (dst & 0x0F) < (src & 0x0F);
    let of = ((dst ^ src) & 0x80) != 0 && ((dst ^ result) & 0x80) != 0;
    (result, cf, of, af)
}

pub fn sub16(dst: u16, src: u16) -> (u16, bool, bool, bool) {
    let result = dst.wrapping_sub(src);
    let cf = dst < src;
    let af = (dst & 0x0F) < (src & 0x0F);
    let of = ((dst ^ src) & 0x8000) != 0 && ((dst ^ result) & 0x8000) != 0;
    (result, cf, of, af)
}

pub fn sub32(dst: u32, src: u32) -> (u32, bool, bool, bool) {
    let result = dst.wrapping_sub(src);
    let cf = dst < src;
    let af = (dst & 0x0F) < (src & 0x0F);
    let of = ((dst ^ src) & 0x8000_0000) != 0 && ((dst ^ result) & 0x8000_0000) != 0;
    (result, cf, of, af)
}

pub fn sbb8(dst: u8, src: u8, carry: bool) -> (u8, bool, bool, bool) {
    let c = if carry { 1u16 } else { 0 };
    let total = src as u16 + c;
    let result = (dst as u16).wrapping_sub(total) as u8;
    let cf = (dst as u16) < total;
    let af = ((dst ^ src ^ result) & 0x10) != 0;
    let of = ((dst ^ src) & 0x80) != 0 && ((dst ^ result) & 0x80) != 0;
    (result, cf, of, af)
}

pub fn sbb16(dst: u16, src: u16, carry: bool) -> (u16, bool, bool, bool) {
    let c = if carry { 1u32 } else { 0 };
    let total = src as u32 + c;
    let result = (dst as u32).wrapping_sub(total) as u16;
    let cf = (dst as u32) < total;
    let af = ((dst ^ src ^ result) & 0x10) != 0;
    let of = ((dst ^ src) & 0x8000) != 0 && ((dst ^ result) & 0x8000) != 0;
    (result, cf, of, af)
}

pub fn sbb32(dst: u32, src: u32, carry: bool) -> (u32, bool, bool, bool) {
    let c = if carry { 1u64 } else { 0 };
    let total = src as u64 + c;
    let result = (dst as u64).wrapping_sub(total) as u32;
    let cf = (dst as u64) < total;
    let af = ((dst ^ src ^ result) & 0x10) != 0;
    let of = ((dst ^ src) & 0x8000_0000) != 0 && ((dst ^ result) & 0x8000_0000) != 0;
    (result, cf, of, af)
}

fn apply_arith8(cpu: &mut X86Cpu, result: u8, cf: bool, of: bool, af: bool) {
    let flags = flags::compute_flags_u8(cpu.registers.flags(), result, cf, of, af);
    set_flags(cpu, flags);
}

fn apply_arith16(cpu: &mut X86Cpu, result: u16, cf: bool, of: bool, af: bool) {
    let flags = flags::compute_flags_u16(cpu.registers.flags(), result, cf, of, af);
    set_flags(cpu, flags);
}

fn apply_arith32(cpu: &mut X86Cpu, result: u32, cf: bool, of: bool, af: bool) {
    let flags = flags::compute_flags_u32(cpu.registers.flags(), result, cf, of, af);
    set_flags(cpu, flags);
}

fn apply_logic8(cpu: &mut X86Cpu, result: u8) {
    set_flags(cpu, flags::compute_logical_flags_u8(cpu.registers.flags(), result));
}

fn apply_logic16(cpu: &mut X86Cpu, result: u16) {
    set_flags(cpu, flags::compute_logical_flags_u16(cpu.registers.flags(), result));
}

fn apply_logic32(cpu: &mut X86Cpu, result: u32) {
    set_flags(cpu, flags::compute_logical_flags_u32(cpu.registers.flags(), result));
}

fn carry(cpu: &X86Cpu) -> bool {
    flags::test_cf(cpu.registers.flags())
}

enum Width {
    B8,
    B16,
    B32,
}

enum Op {
    Add,
    Or,
    Adc,
    Sbb,
    And,
    Sub,
    Xor,
    Cmp,
    Test,
}

fn read_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) -> (u8, Option<u32>) {
    if modrm.is_register_mode() {
        (cpu.read_reg8(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u8(addr), Some(addr))
    }
}

fn write_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, val: u8) {
    if let Some(addr) = addr {
        machine.write_mem_u8(addr, val);
    } else {
        cpu.write_reg8(modrm.rm_field, val);
    }
}

fn read_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) -> (u16, Option<u32>) {
    if modrm.is_register_mode() {
        (cpu.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u16(addr), Some(addr))
    }
}

fn write_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, val: u16) {
    if let Some(addr) = addr {
        machine.write_mem_u16(addr, val);
    } else {
        cpu.write_reg16(modrm.rm_field, val);
    }
}

fn read_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm) -> (u32, Option<u32>) {
    if modrm.is_register_mode() {
        (cpu.read_reg32(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u32(addr), Some(addr))
    }
}

fn write_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: ModRm, addr: Option<u32>, val: u32) {
    if let Some(addr) = addr {
        machine.write_mem_u32(addr, val);
    } else {
        cpu.write_reg32(modrm.rm_field, val);
    }
}

fn binop_rm_r(cpu: &mut X86Cpu, machine: &mut dyn Machine, width: Width, op: Op) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    match width {
        Width::B8 => {
            let src = cpu.read_reg8(modrm.reg_field);
            let (dst, addr) = read_rm8(cpu, machine, modrm);
            let (result, write) = eval8(cpu, op, dst, src);
            if write {
                write_rm8(cpu, machine, modrm, addr, result);
            }
        }
        Width::B16 => {
            let src = cpu.read_reg16(modrm.reg_field);
            let (dst, addr) = read_rm16(cpu, machine, modrm);
            let (result, write) = eval16(cpu, op, dst, src);
            if write {
                write_rm16(cpu, machine, modrm, addr, result);
            }
        }
        Width::B32 => {
            let src = cpu.read_reg32(modrm.reg_field);
            let (dst, addr) = read_rm32(cpu, machine, modrm);
            let (result, write) = eval32(cpu, op, dst, src);
            if write {
                write_rm32(cpu, machine, modrm, addr, result);
            }
        }
    }
}

fn binop_r_rm(cpu: &mut X86Cpu, machine: &mut dyn Machine, width: Width, op: Op) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    match width {
        Width::B8 => {
            let (src, _) = read_rm8(cpu, machine, modrm);
            let dst = cpu.read_reg8(modrm.reg_field);
            let (result, write) = eval8(cpu, op, dst, src);
            if write {
                cpu.write_reg8(modrm.reg_field, result);
            }
        }
        Width::B16 => {
            let (src, _) = read_rm16(cpu, machine, modrm);
            let dst = cpu.read_reg16(modrm.reg_field);
            let (result, write) = eval16(cpu, op, dst, src);
            if write {
                cpu.write_reg16(modrm.reg_field, result);
            }
        }
        Width::B32 => {
            let (src, _) = read_rm32(cpu, machine, modrm);
            let dst = cpu.read_reg32(modrm.reg_field);
            let (result, write) = eval32(cpu, op, dst, src);
            if write {
                cpu.write_reg32(modrm.reg_field, result);
            }
        }
    }
}

fn eval8(cpu: &mut X86Cpu, op: Op, dst: u8, src: u8) -> (u8, bool) {
    match op {
        Op::Add => {
            let (r, cf, of, af) = add8(dst, src);
            apply_arith8(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Adc => {
            let (r, cf, of, af) = adc8(dst, src, carry(cpu));
            apply_arith8(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sub => {
            let (r, cf, of, af) = sub8(dst, src);
            apply_arith8(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sbb => {
            let (r, cf, of, af) = sbb8(dst, src, carry(cpu));
            apply_arith8(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Cmp => {
            let (r, cf, of, af) = sub8(dst, src);
            apply_arith8(cpu, r, cf, of, af);
            (r, false)
        }
        Op::Or => {
            let r = dst | src;
            apply_logic8(cpu, r);
            (r, true)
        }
        Op::And => {
            let r = dst & src;
            apply_logic8(cpu, r);
            (r, true)
        }
        Op::Xor => {
            let r = dst ^ src;
            apply_logic8(cpu, r);
            (r, true)
        }
        Op::Test => {
            let r = dst & src;
            apply_logic8(cpu, r);
            (r, false)
        }
    }
}

fn eval16(cpu: &mut X86Cpu, op: Op, dst: u16, src: u16) -> (u16, bool) {
    match op {
        Op::Add => {
            let (r, cf, of, af) = add16(dst, src);
            apply_arith16(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Adc => {
            let (r, cf, of, af) = adc16(dst, src, carry(cpu));
            apply_arith16(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sub => {
            let (r, cf, of, af) = sub16(dst, src);
            apply_arith16(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sbb => {
            let (r, cf, of, af) = sbb16(dst, src, carry(cpu));
            apply_arith16(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Cmp => {
            let (r, cf, of, af) = sub16(dst, src);
            apply_arith16(cpu, r, cf, of, af);
            (r, false)
        }
        Op::Or => {
            let r = dst | src;
            apply_logic16(cpu, r);
            (r, true)
        }
        Op::And => {
            let r = dst & src;
            apply_logic16(cpu, r);
            (r, true)
        }
        Op::Xor => {
            let r = dst ^ src;
            apply_logic16(cpu, r);
            (r, true)
        }
        Op::Test => {
            let r = dst & src;
            apply_logic16(cpu, r);
            (r, false)
        }
    }
}

fn eval32(cpu: &mut X86Cpu, op: Op, dst: u32, src: u32) -> (u32, bool) {
    match op {
        Op::Add => {
            let (r, cf, of, af) = add32(dst, src);
            apply_arith32(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Adc => {
            let (r, cf, of, af) = adc32(dst, src, carry(cpu));
            apply_arith32(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sub => {
            let (r, cf, of, af) = sub32(dst, src);
            apply_arith32(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Sbb => {
            let (r, cf, of, af) = sbb32(dst, src, carry(cpu));
            apply_arith32(cpu, r, cf, of, af);
            (r, true)
        }
        Op::Cmp => {
            let (r, cf, of, af) = sub32(dst, src);
            apply_arith32(cpu, r, cf, of, af);
            (r, false)
        }
        Op::Or => {
            let r = dst | src;
            apply_logic32(cpu, r);
            (r, true)
        }
        Op::And => {
            let r = dst & src;
            apply_logic32(cpu, r);
            (r, true)
        }
        Op::Xor => {
            let r = dst ^ src;
            apply_logic32(cpu, r);
            (r, true)
        }
        Op::Test => {
            let r = dst & src;
            apply_logic32(cpu, r);
            (r, false)
        }
    }
}

pub fn test_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::Test);
}

pub fn test_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::Test);
}

pub fn test_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::Test);
}

pub fn test_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let r = cpu.registers.al() & imm;
    apply_logic8(cpu, r);
}

pub fn test_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let r = cpu.registers.ax() & imm;
    apply_logic16(cpu, r);
}

pub fn test_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let r = cpu.registers.eax() & imm;
    apply_logic32(cpu, r);
}

pub fn add_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::Add);
}
pub fn add_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::Add);
}
pub fn add_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::Add);
}
pub fn add_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B8, Op::Add);
}
pub fn add_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B16, Op::Add);
}
pub fn add_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B32, Op::Add);
}

pub fn add_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let (r, cf, of, af) = add8(cpu.registers.al(), imm);
    apply_arith8(cpu, r, cf, of, af);
    cpu.registers.set_al(r);
}

pub fn add_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let (r, cf, of, af) = add16(cpu.registers.ax(), imm);
    apply_arith16(cpu, r, cf, of, af);
    cpu.registers.set_ax(r);
}

pub fn add_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let (r, cf, of, af) = add32(cpu.registers.eax(), imm);
    apply_arith32(cpu, r, cf, of, af);
    cpu.registers.set_eax(r);
}

pub fn or_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::Or);
}
pub fn or_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::Or);
}
pub fn or_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::Or);
}
pub fn or_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B8, Op::Or);
}
pub fn or_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B16, Op::Or);
}
pub fn or_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B32, Op::Or);
}

pub fn or_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let r = cpu.registers.al() | imm;
    apply_logic8(cpu, r);
    cpu.registers.set_al(r);
}

pub fn or_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let r = cpu.registers.ax() | imm;
    apply_logic16(cpu, r);
    cpu.registers.set_ax(r);
}

pub fn or_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let r = cpu.registers.eax() | imm;
    apply_logic32(cpu, r);
    cpu.registers.set_eax(r);
}

pub fn adc_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::Adc);
}
pub fn adc_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::Adc);
}
pub fn adc_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::Adc);
}
pub fn adc_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B8, Op::Adc);
}
pub fn adc_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B16, Op::Adc);
}
pub fn adc_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B32, Op::Adc);
}

pub fn adc_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let (r, cf, of, af) = adc8(cpu.registers.al(), imm, carry(cpu));
    apply_arith8(cpu, r, cf, of, af);
    cpu.registers.set_al(r);
}

pub fn adc_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let (r, cf, of, af) = adc16(cpu.registers.ax(), imm, carry(cpu));
    apply_arith16(cpu, r, cf, of, af);
    cpu.registers.set_ax(r);
}

pub fn adc_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let (r, cf, of, af) = adc32(cpu.registers.eax(), imm, carry(cpu));
    apply_arith32(cpu, r, cf, of, af);
    cpu.registers.set_eax(r);
}

pub fn sbb_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::Sbb);
}
pub fn sbb_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::Sbb);
}
pub fn sbb_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::Sbb);
}
pub fn sbb_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B8, Op::Sbb);
}
pub fn sbb_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B16, Op::Sbb);
}
pub fn sbb_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B32, Op::Sbb);
}

pub fn sbb_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let (r, cf, of, af) = sbb8(cpu.registers.al(), imm, carry(cpu));
    apply_arith8(cpu, r, cf, of, af);
    cpu.registers.set_al(r);
}

pub fn sbb_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let (r, cf, of, af) = sbb16(cpu.registers.ax(), imm, carry(cpu));
    apply_arith16(cpu, r, cf, of, af);
    cpu.registers.set_ax(r);
}

pub fn sbb_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let (r, cf, of, af) = sbb32(cpu.registers.eax(), imm, carry(cpu));
    apply_arith32(cpu, r, cf, of, af);
    cpu.registers.set_eax(r);
}

pub fn and_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::And);
}
pub fn and_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::And);
}
pub fn and_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::And);
}
pub fn and_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B8, Op::And);
}
pub fn and_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B16, Op::And);
}
pub fn and_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B32, Op::And);
}

pub fn and_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let r = cpu.registers.al() & imm;
    apply_logic8(cpu, r);
    cpu.registers.set_al(r);
}

pub fn and_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let r = cpu.registers.ax() & imm;
    apply_logic16(cpu, r);
    cpu.registers.set_ax(r);
}

pub fn and_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let r = cpu.registers.eax() & imm;
    apply_logic32(cpu, r);
    cpu.registers.set_eax(r);
}

pub fn sub_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::Sub);
}
pub fn sub_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::Sub);
}
pub fn sub_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::Sub);
}
pub fn sub_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B8, Op::Sub);
}
pub fn sub_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B16, Op::Sub);
}
pub fn sub_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B32, Op::Sub);
}

pub fn sub_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let (r, cf, of, af) = sub8(cpu.registers.al(), imm);
    apply_arith8(cpu, r, cf, of, af);
    cpu.registers.set_al(r);
}

pub fn sub_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let (r, cf, of, af) = sub16(cpu.registers.ax(), imm);
    apply_arith16(cpu, r, cf, of, af);
    cpu.registers.set_ax(r);
}

pub fn sub_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let (r, cf, of, af) = sub32(cpu.registers.eax(), imm);
    apply_arith32(cpu, r, cf, of, af);
    cpu.registers.set_eax(r);
}

pub fn xor_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::Xor);
}
pub fn xor_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::Xor);
}
pub fn xor_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::Xor);
}
pub fn xor_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B8, Op::Xor);
}
pub fn xor_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B16, Op::Xor);
}
pub fn xor_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B32, Op::Xor);
}

pub fn xor_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let r = cpu.registers.al() ^ imm;
    apply_logic8(cpu, r);
    cpu.registers.set_al(r);
}

pub fn xor_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let r = cpu.registers.ax() ^ imm;
    apply_logic16(cpu, r);
    cpu.registers.set_ax(r);
}

pub fn xor_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let r = cpu.registers.eax() ^ imm;
    apply_logic32(cpu, r);
    cpu.registers.set_eax(r);
}

pub fn cmp_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B8, Op::Cmp);
}
pub fn cmp_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B16, Op::Cmp);
}
pub fn cmp_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_rm_r(cpu, machine, Width::B32, Op::Cmp);
}
pub fn cmp_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B8, Op::Cmp);
}
pub fn cmp_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B16, Op::Cmp);
}
pub fn cmp_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    binop_r_rm(cpu, machine, Width::B32, Op::Cmp);
}

pub fn cmp_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine);
    let (r, cf, of, af) = sub8(cpu.registers.al(), imm);
    apply_arith8(cpu, r, cf, of, af);
}

pub fn cmp_ax_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    let (r, cf, of, af) = sub16(cpu.registers.ax(), imm);
    apply_arith16(cpu, r, cf, of, af);
}

pub fn cmp_eax_imm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u32(machine);
    let (r, cf, of, af) = sub32(cpu.registers.eax(), imm);
    apply_arith32(cpu, r, cf, of, af);
}

pub fn daa(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let old_al = cpu.registers.al();
    let old_cf = flags::test_cf(cpu.registers.flags());
    let old_af = flags::test_af(cpu.registers.flags());
    let mut al = old_al;
    let mut cf = false;
    let mut af = false;
    if (old_al & 0x0F) > 9 || old_af {
        al = al.wrapping_add(6);
        af = true;
    }
    if al > 0x9F || old_cf {
        al = al.wrapping_add(0x60);
        cf = true;
    }
    cpu.registers.set_al(al);
    let of = flags::test_of(cpu.registers.flags());
    let f = flags::compute_flags_u8(cpu.registers.flags(), al, cf, of, af);
    set_flags(cpu, f);
}

pub fn das(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let old_al = cpu.registers.al();
    let old_cf = flags::test_cf(cpu.registers.flags());
    let old_af = flags::test_af(cpu.registers.flags());
    let mut al = old_al;
    let mut cf = false;
    let mut af = false;
    if (old_al & 0x0F) > 9 || old_af {
        al = al.wrapping_sub(6);
        af = true;
    }
    if al > 0x9F || old_cf {
        al = al.wrapping_sub(0x60);
        cf = true;
    }
    cpu.registers.set_al(al);
    let of = flags::test_of(cpu.registers.flags());
    let f = flags::compute_flags_u8(cpu.registers.flags(), al, cf, of, af);
    set_flags(cpu, f);
}

fn inc_width(cpu: &mut X86Cpu, reg: u8, bits32: bool) {
    let old_cf = cpu.registers.flags() & flags::CF;
    if bits32 {
        let old = cpu.read_reg32(reg);
        let new = old.wrapping_add(1);
        cpu.write_reg32(reg, new);
        let cf = old == 0xFFFF_FFFF;
        let of = old == 0x7FFF_FFFF;
        let af = (old & 0x0F) == 0x0F;
        let mut f = flags::compute_flags_u32(cpu.registers.flags(), new, cf, of, af);
        f = (f & !flags::CF) | old_cf;
        set_flags(cpu, f);
    } else {
        let old = cpu.read_reg16(reg);
        let new = old.wrapping_add(1);
        cpu.write_reg16(reg, new);
        let cf = old == 0xFFFF;
        let of = old == 0x7FFF;
        let af = (old & 0x0F) == 0x0F;
        let mut f = flags::compute_flags_u16(cpu.registers.flags(), new, cf, of, af);
        f = (f & !flags::CF) | old_cf;
        set_flags(cpu, f);
    }
}

fn dec_width(cpu: &mut X86Cpu, reg: u8, bits32: bool) {
    let old_cf = cpu.registers.flags() & flags::CF;
    if bits32 {
        let old = cpu.read_reg32(reg);
        let new = old.wrapping_sub(1);
        cpu.write_reg32(reg, new);
        let of = old == 0x8000_0000;
        let af = (old & 0x0F) == 0;
        let mut f = flags::compute_flags_u32(cpu.registers.flags(), new, false, of, af);
        f = (f & !flags::CF) | old_cf;
        set_flags(cpu, f);
    } else {
        let old = cpu.read_reg16(reg);
        let new = old.wrapping_sub(1);
        cpu.write_reg16(reg, new);
        let of = old == 0x8000;
        let af = (old & 0x0F) == 0;
        let mut f = flags::compute_flags_u16(cpu.registers.flags(), new, false, of, af);
        f = (f & !flags::CF) | old_cf;
        set_flags(cpu, f);
    }
}

pub fn inc_reg(cpu: &mut X86Cpu, reg: u8) {
    inc_width(cpu, reg, cpu.prefixes.has_operand_size);
}

pub fn dec_reg(cpu: &mut X86Cpu, reg: u8) {
    dec_width(cpu, reg, cpu.prefixes.has_operand_size);
}

pub fn group80(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    group_imm(cpu, machine, false, false);
}

pub fn group81(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    group_imm(cpu, machine, true, false);
}

pub fn group83(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    group_imm(cpu, machine, true, true);
}

fn group_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine, wide: bool, sign8: bool) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let op = match modrm.reg_field {
        0 => Op::Add,
        1 => Op::Or,
        2 => Op::Adc,
        3 => Op::Sbb,
        4 => Op::And,
        5 => Op::Sub,
        6 => Op::Xor,
        _ => Op::Cmp,
    };
    if !wide {
        let (dst, addr) = read_rm8(cpu, machine, modrm);
        let imm = cpu.fetch_u8(machine);
        let (result, write) = eval8(cpu, op, dst, imm);
        if write {
            write_rm8(cpu, machine, modrm, addr, result);
        }
        return;
    }
    let bits32 = cpu.prefixes.has_operand_size;
    if bits32 {
        let (dst, addr) = read_rm32(cpu, machine, modrm);
        let imm = if sign8 {
            cpu.fetch_u8(machine) as i8 as i32 as u32
        } else {
            cpu.fetch_u32(machine)
        };
        let (result, write) = eval32(cpu, op, dst, imm);
        if write {
            write_rm32(cpu, machine, modrm, addr, result);
        }
    } else {
        let (dst, addr) = read_rm16(cpu, machine, modrm);
        let imm = if sign8 {
            cpu.fetch_u8(machine) as i8 as i16 as u16
        } else {
            cpu.fetch_u16(machine)
        };
        let (result, write) = eval16(cpu, op, dst, imm);
        if write {
            write_rm16(cpu, machine, modrm, addr, result);
        }
    }
}
