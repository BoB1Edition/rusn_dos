use crate::cpu::X86Cpu;
use bus::Machine;

use super::core::{add8, add16, add32, adc8, adc16, adc32, sub8, sub16, sub32, sbb8, sbb16, sbb32};
use super::core::{apply_arith8, apply_arith16, apply_arith32, apply_logic8, apply_logic16, apply_logic32, carry, Op, Width};
use super::eval::{binop_rm_r, binop_r_rm};

// TEST
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

// ADD
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

// OR
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

// ADC
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

// SBB
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

// AND
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

// SUB
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

// XOR
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

// CMP
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
