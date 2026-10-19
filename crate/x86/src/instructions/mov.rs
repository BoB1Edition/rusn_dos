// Ver: 5 File: ./crate/x86/src/instructions/mov.rs
use crate::cpu::X86Cpu;
use crate::flags;
use crate::modrm::ModRm;
use bus::Machine;

fn moffs_addr(cpu: &mut X86Cpu, machine: &mut dyn Machine) -> u32 {
    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());

    if cpu.prefixes.has_address_size {
        let offset = cpu.fetch_u32(machine);
        cpu.phys_addr32(seg, offset)
    } else {
        let offset = cpu.fetch_u16(machine);
        cpu.phys_addr(seg, offset)
    }
}

pub fn mov_rm8_r8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = cpu.read_reg8(modrm.reg_field);
    if modrm.is_register_mode() {
        cpu.write_reg8(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u8(phys_addr, src_val);
    }
}

pub fn mov_r8_rm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = if modrm.is_register_mode() {
        cpu.read_reg8(modrm.rm_field)
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u8(phys_addr)
    };
    cpu.write_reg8(modrm.reg_field, src_val);
}

pub fn mov_rm16_r16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = cpu.read_reg16(modrm.reg_field);
    if modrm.is_register_mode() {
        cpu.write_reg16(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u16(phys_addr, src_val);
    }
}

pub fn mov_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = if modrm.is_register_mode() {
        cpu.read_reg16(modrm.rm_field)
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u16(phys_addr)
    };
    cpu.write_reg16(modrm.reg_field, src_val);
}

pub fn mov_rm32_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = cpu.read_reg32(modrm.reg_field);
    if modrm.is_register_mode() {
        cpu.write_reg32(modrm.rm_field, src_val);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u32(phys_addr, src_val);
    }
}

pub fn mov_r32_rm32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = if modrm.is_register_mode() {
        cpu.read_reg32(modrm.rm_field)
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u32(phys_addr)
    };
    cpu.write_reg32(modrm.reg_field, src_val);
}

// === MOV сегментных регистров ===

pub fn mov_rm16_sreg(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let sreg_value = match modrm.reg_field {
        0 => cpu.registers.es(),
        1 => cpu.registers.cs(),
        2 => cpu.registers.ss(),
        3 => cpu.registers.ds(),
        4 => cpu.registers.fs(),
        5 => cpu.registers.gs(),
        _ => unreachable!(),
    };
    if modrm.is_register_mode() {
        cpu.write_reg16(modrm.rm_field, sreg_value);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u16(phys_addr, sreg_value);
    }
}

pub fn mov_sreg_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let src_val = if modrm.is_register_mode() {
        cpu.read_reg16(modrm.rm_field)
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.read_mem_u16(phys_addr)
    };
    match modrm.reg_field {
        0 => cpu.registers.set_es(src_val),
        1 => {
            log::error!("Attempt to write to CS register");
            cpu.halted = true;
        }
        2 => {
            cpu.registers.set_ss(src_val);
            // После MOV SS прерывания подавляются на одну инструкцию
            cpu.inhibit_interrupts = true;
        }
        3 => cpu.registers.set_ds(src_val),
        4 => cpu.registers.set_fs(src_val),
        5 => cpu.registers.set_gs(src_val),
        _ => {
            log::error!("Invalid segment register field");
            cpu.halted = true;
        }
    }
}

// === MOV с непосредственными данными (Immediate) ===

pub fn mov_rm8_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.reg_field != 0 {
        log::error!("MOV r/m8, imm8: reg field {:#x} is not 0", modrm.reg_field);
        cpu.halted = true;
        return;
    }
    if modrm.is_register_mode() {
        let imm8 = cpu.fetch_u8(machine);
        cpu.write_reg8(modrm.rm_field, imm8);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        let imm8 = cpu.fetch_u8(machine);
        machine.write_mem_u8(phys_addr, imm8);
    }
}

pub fn mov_rm_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.reg_field != 0 {
        log::error!("MOV r/m, imm: reg field {:#x} is not 0", modrm.reg_field);
        cpu.halted = true;
        return;
    }
    if cpu.prefixes.has_operand_size {
        if modrm.is_register_mode() {
            let imm = cpu.fetch_u32(machine);
            cpu.write_reg32(modrm.rm_field, imm);
        } else {
            let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            let imm = cpu.fetch_u32(machine);
            machine.write_mem_u32(phys_addr, imm);
        }
    } else if modrm.is_register_mode() {
        let imm = cpu.fetch_u16(machine);
        cpu.write_reg16(modrm.rm_field, imm);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        let imm = cpu.fetch_u16(machine);
        machine.write_mem_u16(phys_addr, imm);
    }
}

pub fn mov_ax_moffs(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    if cpu.prefixes.has_operand_size {
        let val = machine.read_mem_u32(addr);
        cpu.registers.set_eax(val);
    } else {
        let val = machine.read_mem_u16(addr);
        cpu.registers.set_ax(val);
    }
}

pub fn mov_moffs_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    if cpu.prefixes.has_operand_size {
        let val = cpu.registers.eax();
        machine.write_mem_u32(addr, val);
    } else {
        let val = cpu.registers.ax();
        machine.write_mem_u16(addr, val);
    }
}

pub fn lea(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.is_register_mode() {
        log::error!("LEA with register operand");
        cpu.halted = true;
        return;
    }
    let offset = modrm.resolve_offset(cpu, machine, cpu.prefixes.has_address_size);
    if cpu.prefixes.has_operand_size {
        cpu.write_reg32(modrm.reg_field, offset);
    } else {
        cpu.write_reg16(modrm.reg_field, offset as u16);
    }
}

pub fn les(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    load_far(cpu, machine, true);
}

pub fn lds(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    load_far(cpu, machine, false);
}

fn load_far(cpu: &mut X86Cpu, machine: &mut dyn Machine, es: bool) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.is_register_mode() {
        log::error!("LDS/LES with register operand");
        cpu.halted = true;
        return;
    }
    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
    if cpu.prefixes.has_operand_size {
        let off = machine.read_mem_u32(addr);
        let seg = machine.read_mem_u16(addr.wrapping_add(4));
        cpu.write_reg32(modrm.reg_field, off);
        if es {
            cpu.registers.set_es(seg);
        } else {
            cpu.registers.set_ds(seg);
        }
    } else {
        let off = machine.read_mem_u16(addr);
        let seg = machine.read_mem_u16(addr.wrapping_add(2));
        cpu.write_reg16(modrm.reg_field, off);
        if es {
            cpu.registers.set_es(seg);
        } else {
            cpu.registers.set_ds(seg);
        }
    }
}

pub fn mov_al_address(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    let val = machine.read_mem_u8(addr);
    cpu.registers.set_al(val);
}

pub fn mov_ax_address(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    if cpu.prefixes.has_operand_size {
        let val = machine.read_mem_u32(addr);
        cpu.registers.set_eax(val);
    } else {
        let val = machine.read_mem_u16(addr);
        cpu.registers.set_ax(val);
    }
}

pub fn mov_address_al(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    let val = cpu.registers.al();
    machine.write_mem_u8(addr, val);
}

pub fn mov_address_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let addr = moffs_addr(cpu, machine);
    if cpu.prefixes.has_operand_size {
        let val = cpu.registers.eax();
        machine.write_mem_u32(addr, val);
    } else {
        let val = cpu.registers.ax();
        machine.write_mem_u16(addr, val);
    }
}

fn df_set(cpu: &X86Cpu) -> bool {
    (cpu.registers.flags() & flags::DF) != 0
}

// === Хелперы для 16/32-битной адресации строковых операций ===
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
        // 32-битная адресация (префикс 0x67)
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
        // 16-битная адресация (по умолчанию)
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
