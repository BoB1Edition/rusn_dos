// Ver: 1 File: crate/x86/src/instructions/mov.rs
use crate::cpu::X86Cpu;
use crate::flags;
use crate::modrm::ModRm;
use bus::Machine;

// === Базовые MOV регистр-регистр и регистр-память ===

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
            cpu.registers.set_ss(src_val); /* В старом коде был inhibit_interrupts */
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
    let imm8 = cpu.fetch_u8(machine);
    if modrm.is_register_mode() {
        cpu.write_reg8(modrm.rm_field, imm8);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u8(phys_addr, imm8);
    }
}

pub fn mov_rm16_imm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let imm16 = cpu.fetch_u16(machine);
    if modrm.is_register_mode() {
        cpu.write_reg16(modrm.rm_field, imm16);
    } else {
        let phys_addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        machine.write_mem_u16(phys_addr, imm16);
    }
}

// === MOV по прямым адресам (Direct Addressing) ===

pub fn mov_al_address(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let offset = if cpu.prefixes.has_address_size {
        cpu.fetch_u32(machine) as u16
    } else {
        cpu.fetch_u16(machine)
    };
    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    cpu.registers
        .set_al(machine.read_mem_u8(cpu.phys_addr(seg, offset)));
}

pub fn mov_ax_address(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let offset = if cpu.prefixes.has_address_size {
        cpu.fetch_u32(machine) as u16
    } else {
        cpu.fetch_u16(machine)
    };
    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    cpu.registers
        .set_ax(machine.read_mem_u16(cpu.phys_addr(seg, offset)));
}

pub fn mov_address_al(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let offset = if cpu.prefixes.has_address_size {
        cpu.fetch_u32(machine) as u16
    } else {
        cpu.fetch_u16(machine)
    };
    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    machine.write_mem_u8(cpu.phys_addr(seg, offset), cpu.registers.al());
}

pub fn mov_address_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let offset = if cpu.prefixes.has_address_size {
        cpu.fetch_u32(machine) as u16
    } else {
        cpu.fetch_u16(machine)
    };
    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    machine.write_mem_u16(cpu.phys_addr(seg, offset), cpu.registers.ax());
}

// === LEA (Load Effective Address) ===

pub fn lea_r16_rm16(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.is_register_mode() {
        log::warn!("LEA with register mode (mod=11) — emulating as MOV");
        cpu.write_reg16(modrm.reg_field, cpu.read_reg16(modrm.rm_field));
    } else {
        let offset = modrm.resolve_offset(cpu, machine, cpu.prefixes.has_address_size);
        cpu.write_reg16(modrm.reg_field, offset);
    }
}

// === Строковые операции (String Operations) ===

pub fn movsb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let (si, di) = (cpu.registers.si(), cpu.registers.di());
    let val = machine.read_mem_u8(cpu.phys_addr(src_seg, si));
    machine.write_mem_u8(cpu.phys_addr(cpu.registers.es(), di), val);
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_si(if df {
        si.wrapping_sub(1)
    } else {
        si.wrapping_add(1)
    });
    cpu.registers.set_di(if df {
        di.wrapping_sub(1)
    } else {
        di.wrapping_add(1)
    });
}

pub fn movsw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let (si, di) = (cpu.registers.si(), cpu.registers.di());
    let val = machine.read_mem_u16(cpu.phys_addr(src_seg, si));
    machine.write_mem_u16(cpu.phys_addr(cpu.registers.es(), di), val);
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_si(if df {
        si.wrapping_sub(2)
    } else {
        si.wrapping_add(2)
    });
    cpu.registers.set_di(if df {
        di.wrapping_sub(2)
    } else {
        di.wrapping_add(2)
    });
}

pub fn stosb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let di = cpu.registers.di();
    machine.write_mem_u8(cpu.phys_addr(cpu.registers.es(), di), cpu.registers.al());
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_di(if df {
        di.wrapping_sub(1)
    } else {
        di.wrapping_add(1)
    });
}

pub fn stosw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let di = cpu.registers.di();
    machine.write_mem_u16(cpu.phys_addr(cpu.registers.es(), di), cpu.registers.ax());
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_di(if df {
        di.wrapping_sub(2)
    } else {
        di.wrapping_add(2)
    });
}

pub fn lodsb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let si = cpu.registers.si();
    cpu.registers
        .set_al(machine.read_mem_u8(cpu.phys_addr(src_seg, si)));
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_si(if df {
        si.wrapping_sub(1)
    } else {
        si.wrapping_add(1)
    });
}

pub fn cmpsb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let (si, di) = (cpu.registers.si(), cpu.registers.di());
    let src = machine.read_mem_u8(cpu.phys_addr(src_seg, si));
    let dst = machine.read_mem_u8(cpu.phys_addr(cpu.registers.es(), di));
    let result = src.wrapping_sub(dst);
    let cf = src < dst;
    let of = ((src ^ dst) & (src ^ result) & 0x80) != 0;
    let af = (src ^ dst ^ result) & 0x10 != 0;
    cpu.registers.set_flags(flags::compute_flags_u8(
        cpu.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_si(if df {
        si.wrapping_sub(1)
    } else {
        si.wrapping_add(1)
    });
    cpu.registers.set_di(if df {
        di.wrapping_sub(1)
    } else {
        di.wrapping_add(1)
    });
}

pub fn cmpsw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let src_seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let (si, di) = (cpu.registers.si(), cpu.registers.di());
    let src = machine.read_mem_u16(cpu.phys_addr(src_seg, si));
    let dst = machine.read_mem_u16(cpu.phys_addr(cpu.registers.es(), di));
    let result = src.wrapping_sub(dst);
    let cf = src < dst;
    let of = ((src ^ dst) & (src ^ result) & 0x8000) != 0;
    let af = (src ^ dst ^ result) & 0x10 != 0;
    cpu.registers.set_flags(flags::compute_flags_u16(
        cpu.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_si(if df {
        si.wrapping_sub(2)
    } else {
        si.wrapping_add(2)
    });
    cpu.registers.set_di(if df {
        di.wrapping_sub(2)
    } else {
        di.wrapping_add(2)
    });
}

pub fn scasb(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let al = cpu.registers.al();
    let di = cpu.registers.di();
    let src = machine.read_mem_u8(cpu.phys_addr(cpu.registers.es(), di));
    let result = al.wrapping_sub(src);
    let cf = al < src;
    let of = ((al ^ src) & (al ^ result) & 0x80) != 0;
    let af = (al ^ src ^ result) & 0x10 != 0;
    cpu.registers.set_flags(flags::compute_flags_u8(
        cpu.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_di(if df {
        di.wrapping_sub(1)
    } else {
        di.wrapping_add(1)
    });
}

pub fn scasw(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let ax = cpu.registers.ax();
    let di = cpu.registers.di();
    let src = machine.read_mem_u16(cpu.phys_addr(cpu.registers.es(), di));
    let result = ax.wrapping_sub(src);
    let cf = ax < src;
    let of = ((ax ^ src) & (ax ^ result) & 0x8000) != 0;
    let af = (ax ^ src ^ result) & 0x10 != 0;
    cpu.registers.set_flags(flags::compute_flags_u16(
        cpu.registers.flags(),
        result,
        cf,
        of,
        af,
    ));
    let df = (cpu.registers.flags() & flags::DF) != 0;
    cpu.registers.set_di(if df {
        di.wrapping_sub(2)
    } else {
        di.wrapping_add(2)
    });
}
