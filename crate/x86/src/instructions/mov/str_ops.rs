use crate::cpu::X86Cpu;
use crate::flags;
use bus::Machine;

fn df_set(cpu: &X86Cpu) -> bool {
    (cpu.registers.flags() & flags::DF) != 0
}

fn get_src_addr(cpu: &X86Cpu, src_seg: u16) -> u32 {
    if cpu.prefixes.has_address_size {
        cpu.phys_addr32(src_seg, cpu.registers.esi())
    } else {
        cpu.phys_addr(src_seg, cpu.registers.si())
    }
}

fn get_dst_addr(cpu: &X86Cpu, dst_seg: u16) -> u32 {
    if cpu.prefixes.has_address_size {
        cpu.phys_addr32(dst_seg, cpu.registers.edi())
    } else {
        cpu.phys_addr(dst_seg, cpu.registers.di())
    }
}

fn step_index(cpu: &mut X86Cpu, si: bool, di: bool, n: u32) {
    let down = df_set(cpu);
    if cpu.prefixes.has_address_size {
        if si {
            let v = cpu.registers.esi();
            cpu.registers.set_esi(if down {
                v.wrapping_sub(n)
            } else {
                v.wrapping_add(n)
            });
        }
        if di {
            let v = cpu.registers.edi();
            cpu.registers.set_edi(if down {
                v.wrapping_sub(n)
            } else {
                v.wrapping_add(n)
            });
        }
    } else {
        let n16 = n as u16;
        if si {
            let v = cpu.registers.si();
            cpu.registers.set_si(if down {
                v.wrapping_sub(n16)
            } else {
                v.wrapping_add(n16)
            });
        }
        if di {
            let v = cpu.registers.di();
            cpu.registers.set_di(if down {
                v.wrapping_sub(n16)
            } else {
                v.wrapping_add(n16)
            });
        }
    }
}

fn rep_count(cpu: &mut X86Cpu) -> Option<u32> {
    if !cpu.prefixes.has_rep {
        return Some(1);
    }
    let n = if cpu.prefixes.has_address_size {
        cpu.registers.ecx()
    } else {
        cpu.registers.cx() as u32
    };
    if n == 0 { Some(0) } else { Some(n) }
}

fn rep_dec(cpu: &mut X86Cpu) {
    if !cpu.prefixes.has_rep {
        return;
    }
    if cpu.prefixes.has_address_size {
        cpu.registers.set_ecx(cpu.registers.ecx().wrapping_sub(1));
    } else {
        cpu.registers.set_cx(cpu.registers.cx().wrapping_sub(1));
    }
}

pub fn movsb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 {
        return;
    }
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    for _ in 0..count {
        let src_addr = get_src_addr(cpu, src_seg);
        let dst_addr = get_dst_addr(cpu, cpu.registers.es());
        let val = machine.read_mem_u8(src_addr);
        machine.write_mem_u8(dst_addr, val);
        step_index(cpu, true, true, 1);
        rep_dec(cpu);
    }
}

pub fn movsw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 {
        return;
    }
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let is_32bit = cpu.prefixes.has_operand_size;
    let step = if is_32bit { 4u32 } else { 2u32 };

    for _ in 0..count {
        let src_addr = get_src_addr(cpu, src_seg);
        let dst_addr = get_dst_addr(cpu, cpu.registers.es());
        if is_32bit {
            let val = machine.read_mem_u32(src_addr);
            machine.write_mem_u32(dst_addr, val);
        } else {
            let val = machine.read_mem_u16(src_addr);
            machine.write_mem_u16(dst_addr, val);
        }
        step_index(cpu, true, true, step);
        rep_dec(cpu);
    }
}

pub fn cmpsb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    string_cmp(cpu, machine, 1);
}

pub fn cmpsw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let w = if cpu.prefixes.has_operand_size { 4 } else { 2 };
    string_cmp(cpu, machine, w);
}

fn string_cmp(cpu: &mut X86Cpu, machine: &mut dyn Machine, width: u16) {
    let count = match rep_count(cpu) {
        Some(0) => return,
        Some(n) => n,
        None => 1,
    };
    let repne = cpu.prefixes.rep_type == Some(0xF2);
    let repe = cpu.prefixes.rep_type == Some(0xF3);
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());

    for _ in 0..count {
        let src_addr = get_src_addr(cpu, src_seg);
        let dst_addr = get_dst_addr(cpu, cpu.registers.es());

        if width == 1 {
            let src = machine.read_mem_u8(src_addr);
            let dst = machine.read_mem_u8(dst_addr);
            let (result, cf, of, af) = crate::instructions::alu::sub8(src, dst);
            cpu.registers.set_flags(flags::compute_flags_u8(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
        } else if width == 2 {
            let src = machine.read_mem_u16(src_addr);
            let dst = machine.read_mem_u16(dst_addr);
            let (result, cf, of, af) = crate::instructions::alu::sub16(src, dst);
            cpu.registers.set_flags(flags::compute_flags_u16(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
        } else {
            let src = machine.read_mem_u32(src_addr);
            let dst = machine.read_mem_u32(dst_addr);
            let (result, cf, of, af) = crate::instructions::alu::sub32(src, dst);
            cpu.registers.set_flags(flags::compute_flags_u32(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
        }

        step_index(cpu, true, true, width as u32);
        rep_dec(cpu);

        let zf = flags::test_zf(cpu.registers.flags());
        if repe && !zf {
            break;
        }
        if repne && zf {
            break;
        }
    }
}

pub fn stosb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 {
        return;
    }
    for _ in 0..count {
        let dst_addr = get_dst_addr(cpu, cpu.registers.es());
        machine.write_mem_u8(dst_addr, cpu.registers.al());
        step_index(cpu, false, true, 1);
        rep_dec(cpu);
    }
}

pub fn stosw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 {
        return;
    }
    let is_32bit = cpu.prefixes.has_operand_size;
    let step = if is_32bit { 4u32 } else { 2u32 };

    for _ in 0..count {
        let dst_addr = get_dst_addr(cpu, cpu.registers.es());
        if is_32bit {
            machine.write_mem_u32(dst_addr, cpu.registers.eax());
        } else {
            machine.write_mem_u16(dst_addr, cpu.registers.ax());
        }
        step_index(cpu, false, true, step);
        rep_dec(cpu);
    }
}

pub fn lodsb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 {
        return;
    }
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    for _ in 0..count {
        let src_addr = get_src_addr(cpu, src_seg);
        cpu.registers.set_al(machine.read_mem_u8(src_addr));
        step_index(cpu, true, false, 1);
        rep_dec(cpu);
    }
}

pub fn lodsw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 {
        return;
    }
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let is_32bit = cpu.prefixes.has_operand_size;
    let step = if is_32bit { 4u32 } else { 2u32 };

    for _ in 0..count {
        let src_addr = get_src_addr(cpu, src_seg);
        if is_32bit {
            cpu.registers.set_eax(machine.read_mem_u32(src_addr));
        } else {
            cpu.registers.set_ax(machine.read_mem_u16(src_addr));
        }
        step_index(cpu, true, false, step);
        rep_dec(cpu);
    }
}

pub fn scasb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    string_scas(cpu, machine, 1);
}

pub fn scasw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let w = if cpu.prefixes.has_operand_size { 4 } else { 2 };
    string_scas(cpu, machine, w);
}

fn string_scas(cpu: &mut X86Cpu, machine: &mut dyn Machine, width: u16) {
    let count = match rep_count(cpu) {
        Some(0) => return,
        Some(n) => n,
        None => 1,
    };
    let repne = cpu.prefixes.rep_type == Some(0xF2);
    let repe = cpu.prefixes.rep_type == Some(0xF3);

    for _ in 0..count {
        let dst_addr = get_dst_addr(cpu, cpu.registers.es());

        if width == 1 {
            let src = machine.read_mem_u8(dst_addr);
            let (result, cf, of, af) = crate::instructions::alu::sub8(cpu.registers.al(), src);
            cpu.registers.set_flags(flags::compute_flags_u8(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
        } else if width == 2 {
            let src = machine.read_mem_u16(dst_addr);
            let (result, cf, of, af) = crate::instructions::alu::sub16(cpu.registers.ax(), src);
            cpu.registers.set_flags(flags::compute_flags_u16(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
        } else {
            let src = machine.read_mem_u32(dst_addr);
            let (result, cf, of, af) = crate::instructions::alu::sub32(cpu.registers.eax(), src);
            cpu.registers.set_flags(flags::compute_flags_u32(
                cpu.registers.flags(),
                result,
                cf,
                of,
                af,
            ));
        }

        step_index(cpu, false, true, width as u32);
        rep_dec(cpu);

        let zf = flags::test_zf(cpu.registers.flags());
        if repe && !zf {
            break;
        }
        if repne && zf {
            break;
        }
    }
}

pub fn insb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 {
        return;
    }
    let port = cpu.registers.dx();
    for _ in 0..count {
        let val = machine.in_port(port);
        let dst_addr = get_dst_addr(cpu, cpu.registers.es());
        machine.write_mem_u8(dst_addr, val);
        step_index(cpu, false, true, 1);
        rep_dec(cpu);
    }
}

pub fn insw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 { return; }
    let port = cpu.registers.dx();
    let is_32bit = cpu.prefixes.has_operand_size;
    let step = if is_32bit { 4u32 } else { 2u32 };

    for _ in 0..count {
        let dst_addr = get_dst_addr(cpu, cpu.registers.es());
        if is_32bit {
            let val = machine.in_port32(port);
            machine.write_mem_u32(dst_addr, val);
        } else {
            let val = machine.in_port16(port);
            machine.write_mem_u16(dst_addr, val);
        }
        step_index(cpu, false, true, step);
        rep_dec(cpu);
    }
}

pub fn outsb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 {
        return;
    }
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let port = cpu.registers.dx();
    for _ in 0..count {
        let src_addr = get_src_addr(cpu, src_seg);
        let val = machine.read_mem_u8(src_addr);
        machine.out_port(port, val);
        step_index(cpu, true, false, 1);
        rep_dec(cpu);
    }
}

pub fn outsw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let count = rep_count(cpu).unwrap_or(0);
    if count == 0 { return; }
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let port = cpu.registers.dx();
    let is_32bit = cpu.prefixes.has_operand_size;
    let step = if is_32bit { 4u32 } else { 2u32 };

    for _ in 0..count {
        let src_addr = get_src_addr(cpu, src_seg);
        if is_32bit {
            let val = machine.read_mem_u32(src_addr);
            machine.out_port32(port, val);
        } else {
            let val = machine.read_mem_u16(src_addr);
            machine.out_port16(port, val);
        }
        step_index(cpu, true, false, step);
        rep_dec(cpu);
    }
}
