// Ver: 18 File: crate/x86/src/fpu/esc.rs
use crate::cpu::X86Cpu;
use crate::fpu::{F80, arith, control, convert};
use crate::modrm::ModRm;
use bus::Machine;

pub const CR0_MP: u32 = 1 << 1;
pub const CR0_EM: u32 = 1 << 2;
pub const CR0_TS: u32 = 1 << 3;

pub fn execute_esc(cpu: &mut X86Cpu, machine: &mut dyn Machine, opcode: u8) {
    let em = cpu.cr0 & CR0_EM != 0;
    let ts = cpu.cr0 & CR0_TS != 0;

    if em || ts {
        log::error!(
            "FPU ESC {:02X} executed with CR0.EM={} CR0.TS={} -> #NM (Device Not Available)",
            opcode,
            em,
            ts
        );
        cpu.halted = true;
        return;
    }

    let es_before = cpu.fpu.status & crate::fpu::state::STAT_ES != 0;

    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let addr = if !modrm.is_register_mode() {
        Some(modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size))
    } else {
        None
    };

    match opcode {
        0xD8 => dispatch_d8(cpu, machine, &modrm, addr),
        0xD9 => dispatch_d9(cpu, machine, &modrm, addr),
        0xDA => dispatch_da(cpu, machine, &modrm, addr),
        0xDB => dispatch_db(cpu, machine, &modrm, addr),
        0xDC => dispatch_dc(cpu, machine, &modrm, addr),
        0xDD => dispatch_dd(cpu, machine, &modrm, addr),
        0xDE => dispatch_de(cpu, machine, &modrm, addr),
        0xDF => dispatch_df(cpu, machine, &modrm, addr),
        _ => unreachable!("execute_esc вызван для не-ESC опкода {:02X}", opcode),
    }

    let es_after = cpu.fpu.status & crate::fpu::state::STAT_ES != 0;
    if !es_before && es_after {
        log::error!(
            "=== UNMASKED FPU EXCEPTION ===\n\
            Status: {:04X} | Control: {:04X}\n\
            Location: CS:IP = {:04X}:{:04X}\n\
            Instruction bytes: {:02X?}\n\
            =================================",
            cpu.fpu.status,
            cpu.fpu.control,
            cpu.instruction_start_cs,
            cpu.instruction_start_ip,
            cpu.instruction_bytes
        );
        cpu.halted = true;
    }
}

fn not_impl(cpu: &mut X86Cpu, opcode: u8, modrm: &ModRm) {
    let bytes = &cpu.instruction_bytes;
    let bytes_hex = bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    log::error!(
        "=== FPU INSTRUCTION NOT IMPLEMENTED ===\n\
        Opcode: {:02X} | Full Bytes: [ {} ]\n\
        ModRM: mod={} reg={} rm={} (is_reg_mode={})\n\
        Location: CS:IP = {:04X}:{:04X}\n\
        FPU State: TOP={} | Status={:04X} | Control={:04X}\n\
        ST(0)={:} | ST(1)={:} | ST(2)={:}\n\
        =======================================",
        opcode,
        bytes_hex,
        modrm.mod_field,
        modrm.reg_field,
        modrm.rm_field,
        modrm.is_register_mode(),
        cpu.instruction_start_cs,
        cpu.instruction_start_ip,
        cpu.fpu.top(),
        cpu.fpu.status,
        cpu.fpu.control,
        cpu.fpu.st(0).hex(),
        cpu.fpu.st(1).hex(),
        cpu.fpu.st(2).hex()
    );
    cpu.halted = true;
}

fn dispatch_d8(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: &ModRm, addr: Option<u32>) {
    let rm = cpu.fpu.rounding_mode();

    if modrm.is_register_mode() {
        let i = modrm.rm_field as usize;
        if !cpu.fpu.check_st(i) {
            return;
        }

        let st0 = cpu.fpu.st(0);
        let sti = cpu.fpu.st(i);

        let (result, flags) = match modrm.reg_field {
            0 => arith::add(st0, sti, rm),
            1 => arith::mul(st0, sti, rm),
            2 => {
                let (cmp_result, cmp_flags) = arith::cmp(st0, sti);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                return;
            }
            3 => {
                let (cmp_result, cmp_flags) = arith::cmp(st0, sti);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                let _ = cpu.fpu.pop();
                return;
            }
            4 => arith::sub(st0, sti, rm),
            5 => arith::sub(sti, st0, rm),
            6 => arith::div(st0, sti, rm),
            7 => arith::div(sti, st0, rm),
            _ => unreachable!(),
        };

        cpu.fpu.set_st(0, result);
        cpu.fpu.raise(flags);
    } else {
        let addr = addr.expect("Memory form must have address");
        let m32_bits = machine.read_mem_u32(addr);
        let m32 = convert::from_f32_bits(m32_bits);

        let st0 = cpu.fpu.st(0);

        let (result, flags) = match modrm.reg_field {
            0 => arith::add(st0, m32, rm),
            1 => arith::mul(st0, m32, rm),
            2 => {
                let (cmp_result, cmp_flags) = arith::cmp(st0, m32);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                return;
            }
            3 => {
                let (cmp_result, cmp_flags) = arith::cmp(st0, m32);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                let _ = cpu.fpu.pop();
                return;
            }
            4 => arith::sub(st0, m32, rm),
            5 => arith::sub(m32, st0, rm),
            6 => arith::div(st0, m32, rm),
            7 => arith::div(m32, st0, rm),
            _ => unreachable!(),
        };

        cpu.fpu.set_st(0, result);
        cpu.fpu.raise(flags);
    }
}

fn dispatch_d9(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: &ModRm, addr: Option<u32>) {
    if modrm.is_register_mode() {
        dispatch_d9_reg(cpu, modrm);
    } else {
        dispatch_d9_mem(cpu, machine, modrm, addr);
    }
}

fn dispatch_d9_mem(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: &ModRm, addr: Option<u32>) {
    let addr = addr.expect("D9 memory form must have address");
    match modrm.reg_field {
        0 => {
            let bits = machine.read_mem_u32(addr);
            let val = convert::from_f32_bits(bits);
            let _ = cpu.fpu.push(val);
        }
        2 => {
            if !cpu.fpu.check_st(0) {
                return;
            }
            let st0 = cpu.fpu.st(0);
            let (bits, flags) = convert::to_f32_bits(st0, cpu.fpu.rounding_mode());
            machine.write_mem_u32(addr, bits);
            cpu.fpu.raise(flags);
        }
        3 => {
            if !cpu.fpu.check_st(0) {
                return;
            }
            let st0 = cpu.fpu.st(0);
            let (bits, flags) = convert::to_f32_bits(st0, cpu.fpu.rounding_mode());
            machine.write_mem_u32(addr, bits);
            cpu.fpu.raise(flags);
            let _ = cpu.fpu.pop();
        }
        5 => control::fldcw(cpu, machine, addr),
        7 => control::fnstcw(cpu, machine, addr),
        _ => not_impl(cpu, 0xD9, modrm),
    }
}

fn dispatch_d9_reg(cpu: &mut X86Cpu, modrm: &ModRm) {
    match modrm.reg_field {
        0 => {
            let i = modrm.rm_field as usize;
            if !cpu.fpu.check_st(i) {
                return;
            }
            let val = cpu.fpu.st(i);
            let _ = cpu.fpu.push(val);
        }
        1 => {
            let i = modrm.rm_field as usize;
            if !cpu.fpu.check_st(0) || !cpu.fpu.check_st(i) {
                return;
            }
            let st0 = cpu.fpu.st(0);
            let sti = cpu.fpu.st(i);
            cpu.fpu.set_st(0, sti);
            cpu.fpu.set_st(i, st0);
        }
        2 => {
            if modrm.rm_field != 0 {
                not_impl(cpu, 0xD9, modrm);
            }
        }
        _ => dispatch_d9_unary(cpu, modrm),
    }
}

fn dispatch_d9_unary(cpu: &mut X86Cpu, modrm: &ModRm) {
    let op = (modrm.reg_field << 3) | modrm.rm_field;
    match op {
        0x20 => {
            if !cpu.fpu.check_st(0) {
                return;
            }
            let result = arith::neg(cpu.fpu.st(0));
            cpu.fpu.set_st(0, result);
        }
        0x21 => {
            if !cpu.fpu.check_st(0) {
                return;
            }
            let st0 = cpu.fpu.st(0);
            let result = F80::from_parts(false, st0.biased_exp(), st0.mant);
            cpu.fpu.set_st(0, result);
        }
        0x24 => {
            if !cpu.fpu.check_st(0) {
                return;
            }
            let (cmp_result, cmp_flags) = arith::ftst(cpu.fpu.st(0));
            cpu.fpu.set_cc_from_cmp(cmp_result);
            cpu.fpu.raise(cmp_flags);
        }
        0x25 => {
            if cpu.fpu.st_tag(0) == crate::fpu::state::Tag::Empty {
                cpu.fpu.set_cc(true, false, false, true); // Empty: C3=1, C2=0, C0=1, C1=0
                return;
            }
            let st0 = cpu.fpu.st(0);
            let (c3, c2, c0) = match st0.classify() {
                crate::fpu::f80::F80Class::Unnormal
                | crate::fpu::f80::F80Class::PseudoInfinity
                | crate::fpu::f80::F80Class::PseudoNaN => (false, false, false),
                crate::fpu::f80::F80Class::QNaN | crate::fpu::f80::F80Class::SNaN => {
                    (false, false, true)
                }
                crate::fpu::f80::F80Class::Normal => (false, true, false),
                crate::fpu::f80::F80Class::Infinity => (false, true, true),
                crate::fpu::f80::F80Class::Zero => (true, false, false),
                crate::fpu::f80::F80Class::Subnormal | crate::fpu::f80::F80Class::Denormal => {
                    (true, true, false)
                }
            };
            cpu.fpu.set_cc(c3, c2, st0.sign(), c0);
        }
        0x28 => {
            let _ = cpu.fpu.push(F80::one(false));
        }
        0x29 => {
            let _ = cpu.fpu.push(F80::log2_10());
        }
        0x2A => {
            let _ = cpu.fpu.push(F80::log2_e());
        }
        0x2B => {
            let _ = cpu.fpu.push(F80::pi());
        }
        0x2C => {
            let _ = cpu.fpu.push(F80::log10_2());
        }
        0x2D => {
            let _ = cpu.fpu.push(F80::ln2());
        }
        0x2E => {
            let _ = cpu.fpu.push(F80::zero(false));
        }
        0x3A => {
            if !cpu.fpu.check_st(0) {
                return;
            }
            let (result, flags) = arith::sqrt(cpu.fpu.st(0), cpu.fpu.rounding_mode());
            cpu.fpu.set_st(0, result);
            cpu.fpu.raise(flags);
        }
        0x3C => {
            if !cpu.fpu.check_st(0) {
                return;
            }
            let (result, flags) = arith::round_to_integer(cpu.fpu.st(0), cpu.fpu.rounding_mode());
            cpu.fpu.set_st(0, result);
            cpu.fpu.raise(flags);
        }
        _ => not_impl(cpu, 0xD9, modrm),
    }
}

fn dispatch_da(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    not_impl(cpu, 0xDA, modrm);
}

fn dispatch_db(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    match modrm.reg_field {
        4 => {
            if modrm.is_register_mode() && modrm.rm_field == 3 {
                control::fninit(cpu);
            } else {
                not_impl(cpu, 0xDB, modrm);
            }
        }
        _ => not_impl(cpu, 0xDB, modrm),
    }
}

fn dispatch_dc(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: &ModRm, addr: Option<u32>) {
    let rm = cpu.fpu.rounding_mode();

    if modrm.is_register_mode() {
        let i = modrm.rm_field as usize;
        if !cpu.fpu.check_st(i) {
            return;
        }
        let st0 = cpu.fpu.st(0);
        let sti = cpu.fpu.st(i);

        let (result, flags) = match modrm.reg_field {
            0 => arith::add(sti, st0, rm),
            1 => arith::mul(sti, st0, rm),
            2 => {
                let (cmp_result, cmp_flags) = arith::cmp(st0, sti);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                return;
            }
            3 => {
                let (cmp_result, cmp_flags) = arith::cmp(st0, sti);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                let _ = cpu.fpu.pop();
                return;
            }
            4 => arith::sub(st0, sti, rm),
            5 => arith::sub(sti, st0, rm),
            6 => arith::div(st0, sti, rm),
            7 => arith::div(sti, st0, rm),
            _ => unreachable!(),
        };

        cpu.fpu.set_st(i, result);
        cpu.fpu.raise(flags);
    } else {
        let addr = addr.expect("Memory form must have address");
        let lo = machine.read_mem_u32(addr);
        let hi = machine.read_mem_u32(addr.wrapping_add(4));
        let m64_bits = (lo as u64) | ((hi as u64) << 32);
        let m64 = convert::from_f64_bits(m64_bits);
        let st0 = cpu.fpu.st(0);

        let (result, flags) = match modrm.reg_field {
            0 => arith::add(st0, m64, rm),
            1 => arith::mul(st0, m64, rm),
            2 => {
                let (cmp_result, cmp_flags) = arith::cmp(st0, m64);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                return;
            }
            3 => {
                let (cmp_result, cmp_flags) = arith::cmp(st0, m64);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                let _ = cpu.fpu.pop();
                return;
            }
            4 => arith::sub(st0, m64, rm),
            5 => arith::sub(m64, st0, rm),
            6 => arith::div(st0, m64, rm),
            7 => arith::div(m64, st0, rm),
            _ => unreachable!(),
        };

        cpu.fpu.set_st(0, result);
        cpu.fpu.raise(flags);
    }
}

fn dispatch_dd(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: &ModRm, addr: Option<u32>) {
    if modrm.is_register_mode() {
        let i = modrm.rm_field as usize;

        match modrm.reg_field {
            0 => {
                let phys = cpu.fpu.phys(i);
                cpu.fpu.set_tag(phys, crate::fpu::state::Tag::Empty);
            }

            2 => {
                if !cpu.fpu.check_st(0) {
                    return;
                }

                let st0 = cpu.fpu.st(0);
                cpu.fpu.set_st(i, st0);
            }

            3 => {
                if !cpu.fpu.check_st(0) {
                    return;
                }

                let st0 = cpu.fpu.st(0);
                cpu.fpu.set_st(i, st0);
                let _ = cpu.fpu.pop();
            }

            4 => {
                if !cpu.fpu.check_st(0) || !cpu.fpu.check_st(i) {
                    return;
                }

                let st0 = cpu.fpu.st(0);
                let sti = cpu.fpu.st(i);
                let (cmp_result, cmp_flags) = arith::cmp(st0, sti);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
            }

            5 => {
                if !cpu.fpu.check_st(0) || !cpu.fpu.check_st(i) {
                    return;
                }

                let st0 = cpu.fpu.st(0);
                let sti = cpu.fpu.st(i);
                let (cmp_result, cmp_flags) = arith::cmp(st0, sti);
                cpu.fpu.set_cc_from_cmp(cmp_result);
                cpu.fpu.raise(cmp_flags);
                let _ = cpu.fpu.pop();
            }

            _ => {
                not_impl(cpu, 0xDD, modrm);
            }
        }
    } else {
        let addr = addr.expect("Memory form must have address");

        match modrm.reg_field {
            0 => {
                let lo = machine.read_mem_u32(addr);
                let hi = machine.read_mem_u32(addr.wrapping_add(4));
                let bits = (lo as u64) | ((hi as u64) << 32);
                let val = convert::from_f64_bits(bits);
                let _ = cpu.fpu.push(val);
            }

            2 => {
                if !cpu.fpu.check_st(0) {
                    return;
                }

                let st0 = cpu.fpu.st(0);
                let (bits, flags) = convert::to_f64_bits(st0, cpu.fpu.rounding_mode());

                machine.write_mem_u32(addr, bits as u32);
                machine.write_mem_u32(addr.wrapping_add(4), (bits >> 32) as u32);

                cpu.fpu.raise(flags);
            }

            3 => {
                if !cpu.fpu.check_st(0) {
                    return;
                }

                let st0 = cpu.fpu.st(0);
                let (bits, flags) = convert::to_f64_bits(st0, cpu.fpu.rounding_mode());

                machine.write_mem_u32(addr, bits as u32);
                machine.write_mem_u32(addr.wrapping_add(4), (bits >> 32) as u32);

                cpu.fpu.raise(flags);
                let _ = cpu.fpu.pop();
            }

            7 => {
                machine.write_mem_u16(addr, cpu.fpu.status);
            }

            _ => {
                not_impl(cpu, 0xDD, modrm);
            }
        }
    }
}

fn dispatch_de(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    not_impl(cpu, 0xDE, modrm);
}

fn dispatch_df(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: &ModRm, addr: Option<u32>) {
    match modrm.reg_field {
        4 => {
            if modrm.is_register_mode() && modrm.rm_field == 0 {
                control::fnstsw_ax(cpu);
            } else {
                not_impl(cpu, 0xDF, modrm);
            }
        }
        _ => not_impl(cpu, 0xDF, modrm),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpu::F80;
    use crate::fpu::state::CTRL_DEFAULT;
    use bus::Motherboard;

    fn fresh_cpu() -> X86Cpu {
        let mut cpu = X86Cpu::new();
        cpu.registers.set_cs(0);
        cpu.registers.set_eip(0);
        cpu
    }

    #[test]
    fn test_em_blocks_esc() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.cr0 = CR0_EM;
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(cpu.halted, "CR0.EM must block FPU ESC");
        assert_eq!(cpu.registers.eip(), 0);
        assert_eq!(cpu.fpu.control, CTRL_DEFAULT);
    }

    #[test]
    fn test_ts_blocks_esc_db() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.cr0 = CR0_TS;
        execute_esc(&mut cpu, &mut mb, 0xDB);
        assert!(cpu.halted, "CR0.TS must block FPU ESC");
        assert_eq!(cpu.registers.eip(), 0);
    }

    #[test]
    fn test_d9_ts_blocks_esc() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.cr0 = CR0_TS;
        mb.write_mem_u8(0, 0xE0);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(cpu.halted, "D9 must block on CR0.TS");
        assert_eq!(cpu.registers.eip(), 0, "ModRM не должен читаться");
        assert_eq!(cpu.fpu.control, CTRL_DEFAULT);
    }

    #[test]
    fn test_fldcw_memory() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.registers.set_bx(0);
        cpu.registers.set_si(0);
        mb.write_mem_u16(0x10, 0x1234);
        mb.write_mem_u8(0, 0x68);
        mb.write_mem_u8(1, 0x10);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.control, 0x1234);
        assert_eq!(cpu.registers.eip(), 2, "ModRM + disp8");
    }

    #[test]
    fn test_fnstcw_memory() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.registers.set_bx(0);
        cpu.registers.set_si(0);
        cpu.fpu.control = 0xABCD;
        mb.write_mem_u8(0, 0x78);
        mb.write_mem_u8(1, 0x10);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(mb.read_mem_u16(0x10), 0xABCD);
        assert_eq!(cpu.registers.eip(), 2);
    }

    #[test]
    fn test_dc_fadd_st1_st0() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();

        mb.write_mem_u8(0, 0xC1);
        execute_esc(&mut cpu, &mut mb, 0xDC);
        assert!(!cpu.halted);

        let st0 = cpu.fpu.st(0);
        let st1 = cpu.fpu.st(1);
        assert_eq!(st0.biased_exp(), 16383);
        assert_eq!(st0.mant, 1u64 << 63);
        assert_eq!(st1.biased_exp(), 16384);
        assert_eq!(st1.mant, 0xC000_0000_0000_0000);
    }

    #[test]
    fn test_dc_fadd_m64() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.registers.set_bx(0);
        cpu.registers.set_si(0);

        cpu.fpu.push(F80::one(false)).unwrap();

        mb.write_mem_u32(0x10, 0x0000_0000);
        mb.write_mem_u32(0x14, 0x4000_0000);
        mb.write_mem_u8(0, 0x40);
        mb.write_mem_u8(1, 0x10);

        execute_esc(&mut cpu, &mut mb, 0xDC);
        assert!(!cpu.halted);
        let result = cpu.fpu.st(0);
        assert_eq!(result.biased_exp(), 16384);
        assert_eq!(result.mant, 0xC000_0000_0000_0000);
    }

    #[test]
    fn test_dc_fcom_st0_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();

        mb.write_mem_u8(0, 0xD1);
        execute_esc(&mut cpu, &mut mb, 0xDC);
        assert!(!cpu.halted);

        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C3, 0);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C2, 0);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C1, 0);
        assert_ne!(cpu.fpu.status & crate::fpu::state::STAT_C0, 0);

        assert_eq!(cpu.fpu.top(), 6);
    }

    #[test]
    fn test_dc_fsub_st1_st0() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16384, 0xC000_0000_0000_0000))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();
        mb.write_mem_u8(0, 0xE9);
        execute_esc(&mut cpu, &mut mb, 0xDC);
        assert!(!cpu.halted);
        let st1 = cpu.fpu.st(1);
        assert!(!st1.sign(), "FSUB: 3.0 - 1.0 должен быть положительным");
        assert_eq!(st1.biased_exp(), 16384);
        assert_eq!(st1.mant, 1u64 << 63);
    }

    #[test]
    fn test_dc_fdiv_st1_st0() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16385, 1u64 << 63))
            .unwrap();
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        mb.write_mem_u8(0, 0xF9);
        execute_esc(&mut cpu, &mut mb, 0xDC);
        assert!(!cpu.halted);
        let st1 = cpu.fpu.st(1);
        assert!(!st1.sign(), "FDIV: 4.0 / 2.0 должен быть положительным");
        assert_eq!(st1.biased_exp(), 16384);
        assert_eq!(st1.mant, 1u64 << 63);
    }

    #[test]
    fn test_fnstsw_m16_memory() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.registers.set_bx(0);
        cpu.registers.set_si(0);
        cpu.fpu.status = 0x0034;
        mb.write_mem_u8(0, 0x78);
        mb.write_mem_u8(1, 0x10);
        execute_esc(&mut cpu, &mut mb, 0xDD);
        assert!(!cpu.halted);
        assert_eq!(mb.read_mem_u16(0x10), 0x0034);
        assert_eq!(cpu.registers.eip(), 2);
    }

    #[test]
    fn test_fninit_db_e3() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(false)).unwrap();
        cpu.fpu.status = 0xFFFF;
        cpu.fpu.control = 0;
        mb.write_mem_u8(0, 0xE3);
        execute_esc(&mut cpu, &mut mb, 0xDB);
        assert!(!cpu.halted, "FNINIT должен выполниться");
        assert_eq!(cpu.fpu.status, 0);
        assert_eq!(cpu.fpu.control, CTRL_DEFAULT);
        assert_eq!(cpu.fpu.top(), 0);
        assert_eq!(cpu.registers.eip(), 1);
    }

    #[test]
    fn test_fnstsw_ax_df_e0() {
        use crate::fpu::state::{STAT_C0, STAT_PE};
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        let status = STAT_C0 | STAT_PE | (3u16 << 11);
        cpu.fpu.status = status;
        mb.write_mem_u8(0, 0xE0);
        execute_esc(&mut cpu, &mut mb, 0xDF);
        assert!(!cpu.halted, "FNSTSW AX не должен вызывать halt");
        assert_eq!(cpu.registers.ax(), status);
        assert_eq!(cpu.fpu.status, status);
        assert_eq!(cpu.registers.eip(), 1);
    }

    #[test]
    fn test_d8_fadd_st0_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(false)).unwrap();
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();

        mb.write_mem_u8(0, 0xC1);
        execute_esc(&mut cpu, &mut mb, 0xD8);
        assert!(!cpu.halted);
        let result = cpu.fpu.st(0);
        assert_eq!(result.classify(), crate::fpu::f80::F80Class::Normal);
        assert_eq!(result.biased_exp(), 16384);
        assert_eq!(result.mant, 0xC000_0000_0000_0000);
    }

    #[test]
    fn test_d8_fadd_m32() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.registers.set_bx(0);
        cpu.registers.set_si(0);
        cpu.fpu.push(F80::one(false)).unwrap();
        mb.write_mem_u32(0x10, 0x4000_0000);
        mb.write_mem_u8(0, 0x40);
        mb.write_mem_u8(1, 0x10);

        execute_esc(&mut cpu, &mut mb, 0xD8);
        assert!(!cpu.halted);
        let result = cpu.fpu.st(0);
        assert_eq!(result.biased_exp(), 16384);
        assert_eq!(result.mant, 0xC000_0000_0000_0000);
    }

    #[test]
    fn test_d8_fcom_st0_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(false)).unwrap();
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();

        mb.write_mem_u8(0, 0xD1);
        execute_esc(&mut cpu, &mut mb, 0xD8);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C3, 0);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C2, 0);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C1, 0);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C0, 0);
        assert_eq!(cpu.fpu.top(), 6);
    }

    #[test]
    fn test_d8_fcomp_st0_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();

        let top_before = cpu.fpu.top();
        mb.write_mem_u8(0, 0xD9);
        execute_esc(&mut cpu, &mut mb, 0xD8);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C3, 0);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C2, 0);
        assert_eq!(cpu.fpu.status & crate::fpu::state::STAT_C1, 0);
        assert_ne!(cpu.fpu.status & crate::fpu::state::STAT_C0, 0);
        assert_eq!(cpu.fpu.top(), (top_before + 1) & 7);
    }

    #[test]
    fn test_d8_fsub_st0_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(false)).unwrap();
        cpu.fpu
            .push(F80::from_parts(false, 16384, 0xC000_0000_0000_0000))
            .unwrap();

        mb.write_mem_u8(0, 0xE1);
        execute_esc(&mut cpu, &mut mb, 0xD8);
        assert!(!cpu.halted);
        let result = cpu.fpu.st(0);
        assert_eq!(result.biased_exp(), 16384);
        assert_eq!(result.mant, 1u64 << 63);
    }

    #[test]
    fn test_d8_fdiv_st0_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        cpu.fpu
            .push(F80::from_parts(false, 16385, 1u64 << 63))
            .unwrap();

        mb.write_mem_u8(0, 0xF1);
        execute_esc(&mut cpu, &mut mb, 0xD8);
        assert!(!cpu.halted);
        let result = cpu.fpu.st(0);
        assert_eq!(result.biased_exp(), 16384);
        assert_eq!(result.mant, 1u64 << 63);
    }

    #[test]
    fn test_dd_ffree_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);

        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();

        let top_before = cpu.fpu.top();

        mb.write_mem_u8(0, 0xC1);
        execute_esc(&mut cpu, &mut mb, 0xDD);

        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.top(), top_before);
        assert_eq!(cpu.fpu.st_tag(1), crate::fpu::state::Tag::Empty);
    }

    #[test]
    fn test_dd_fst_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);

        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();

        let top_before = cpu.fpu.top();

        mb.write_mem_u8(0, 0xD1);
        execute_esc(&mut cpu, &mut mb, 0xDD);

        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.top(), top_before);

        assert_eq!(cpu.fpu.st(0), F80::one(false));
        assert_eq!(cpu.fpu.st(1), F80::one(false));
    }

    #[test]
    fn test_dd_fstp_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);

        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();

        let top_before = cpu.fpu.top();

        mb.write_mem_u8(0, 0xD9);
        execute_esc(&mut cpu, &mut mb, 0xDD);

        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.top(), (top_before + 1) & 7);
        assert_eq!(cpu.fpu.st(0), F80::one(false));
    }

    #[test]
    fn test_dc_fsubr_st1_st0() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16384, 0xC000_0000_0000_0000))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();
        mb.write_mem_u8(0, 0xE1);
        execute_esc(&mut cpu, &mut mb, 0xDC);
        assert!(!cpu.halted);
        let st1 = cpu.fpu.st(1);
        assert!(st1.sign(), "FSUBR: 1.0 - 3.0 должен быть отрицательным");
        assert_eq!(st1.biased_exp(), 16384);
        assert_eq!(st1.mant, 1u64 << 63);
    }

    #[test]
    fn test_dc_fdivr_st1_st0() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16385, 1u64 << 63))
            .unwrap();
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        mb.write_mem_u8(0, 0xF1);
        execute_esc(&mut cpu, &mut mb, 0xDC);
        assert!(!cpu.halted);
        let st1 = cpu.fpu.st(1);
        assert!(!st1.sign(), "FDIVR: 2.0 / 4.0 должен быть положительным");
        assert_eq!(st1.biased_exp(), 16382);
        assert_eq!(st1.mant, 1u64 << 63);
    }
    #[test]
    fn test_d9_fld_m32() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.registers.set_bx(0);
        cpu.registers.set_si(0);
        mb.write_mem_u32(0x10, 0x4000_0000);
        mb.write_mem_u8(0, 0x40);
        mb.write_mem_u8(1, 0x10);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0).biased_exp(), 16384);
        assert_eq!(cpu.fpu.st(0).mant, 1u64 << 63);
    }

    #[test]
    fn test_d9_fld_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        let top_before = cpu.fpu.top();
        mb.write_mem_u8(0, 0xC0);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.top(), (top_before + 7) & 7);
        assert_eq!(cpu.fpu.st(0), F80::from_parts(false, 16384, 1u64 << 63));
        assert_eq!(cpu.fpu.st(1), F80::from_parts(false, 16384, 1u64 << 63));
    }

    #[test]
    fn test_d9_fxch_st1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16384, 1u64 << 63))
            .unwrap();
        cpu.fpu.push(F80::one(false)).unwrap();
        mb.write_mem_u8(0, 0xC9);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0), F80::from_parts(false, 16384, 1u64 << 63));
        assert_eq!(cpu.fpu.st(1), F80::one(false));
    }

    #[test]
    fn test_d9_fchs() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(false)).unwrap();
        mb.write_mem_u8(0, 0xE0);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0), F80::one(true));
    }

    #[test]
    fn test_d9_fabs() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(true)).unwrap();
        mb.write_mem_u8(0, 0xE1);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0), F80::one(false));
    }

    #[test]
    fn test_d9_fld1() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        mb.write_mem_u8(0, 0xE8);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0), F80::one(false));
    }

    #[test]
    fn test_d9_fldz() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        mb.write_mem_u8(0, 0xEE);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0).classify(), crate::fpu::f80::F80Class::Zero);
    }

    #[test]
    fn test_d9_fldpi() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        mb.write_mem_u8(0, 0xEB);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0).biased_exp(), 0x4000);
        assert_eq!(cpu.fpu.st(0).mant, 0xC90F_DAA2_2168_C235);
    }

    #[test]
    fn test_d9_fsqrt() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu
            .push(F80::from_parts(false, 16385, 1u64 << 63))
            .unwrap();
        mb.write_mem_u8(0, 0xFA);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0).biased_exp(), 16384);
        assert_eq!(cpu.fpu.st(0).mant, 1u64 << 63);
    }

    #[test]
    fn test_d9_frndint() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(false)).unwrap();
        cpu.fpu.pop().unwrap();
        cpu.fpu
            .push(F80::from_parts(false, 16384, 0xA000_0000_0000_0000))
            .unwrap();
        mb.write_mem_u8(0, 0xFC);
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(cpu.fpu.st(0).biased_exp(), 16384);
        assert_eq!(cpu.fpu.st(0).mant, 1u64 << 63);
    }
}
