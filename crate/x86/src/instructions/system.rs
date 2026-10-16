// Ver: 2 File: crate/x86/src/instructions/system.rs

use bus::Machine;
use crate::cpu::X86Cpu;
use crate::flags;

pub fn nop(_cpu: &mut X86Cpu, _machine: &mut dyn Machine) {}

pub fn hlt(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    log::info!("HLT executed. Halting CPU.");
    cpu.halted = true;
}

pub fn int(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let vector = cpu.fetch_u8(machine);
    software_interrupt(cpu, machine, vector);
}

pub fn software_interrupt(cpu: &mut X86Cpu, machine: &mut dyn Machine, vector: u8) {
    if machine.handle_software_interrupt(vector) {
        // Шина (DOS-слой) обработает прерывание сама, но ей нужен доступ к
        // регистрам CPU. Поэтому запоминаем вектор: драйвер вызовет обработчик
        // после возврата из `step`, когда borrow CPU уже освобождён.
        cpu.pending_interrupt = Some(vector);
        return;
    }

    let ivt_addr = (vector as u32) * 4;
    let handler_ip = machine.read_mem_u16(ivt_addr);
    let handler_cs = machine.read_mem_u16(ivt_addr + 2);

    let ss_base = cpu.phys_addr(cpu.registers.ss(), 0);
    let mut sp = cpu.registers.sp();

    sp = sp.wrapping_sub(2);
    machine.write_mem_u16(ss_base.wrapping_add(sp as u32), cpu.registers.flags());
    sp = sp.wrapping_sub(2);
    machine.write_mem_u16(ss_base.wrapping_add(sp as u32), cpu.registers.cs());
    sp = sp.wrapping_sub(2);
    machine.write_mem_u16(ss_base.wrapping_add(sp as u32), cpu.registers.ip());

    cpu.registers.set_sp(sp);

    let mut f = cpu.registers.flags();
    f &= !(flags::IF | flags::TF);
    cpu.registers.set_flags(f);

    cpu.registers.set_cs(handler_cs);
    cpu.registers.set_ip(handler_ip);
}

pub fn int3(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    software_interrupt(cpu, machine, 3);
}

pub fn iret(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let ss_base = cpu.phys_addr(cpu.registers.ss(), 0);
    let mut sp = cpu.registers.sp();

    let ip = machine.read_mem_u16(ss_base.wrapping_add(sp as u32));
    sp = sp.wrapping_add(2);
    let cs = machine.read_mem_u16(ss_base.wrapping_add(sp as u32));
    sp = sp.wrapping_add(2);
    let f = machine.read_mem_u16(ss_base.wrapping_add(sp as u32));
    sp = sp.wrapping_add(2);

    cpu.registers.set_ip(ip);
    cpu.registers.set_cs(cs);
    cpu.registers.set_flags(f);
    cpu.registers.set_sp(sp);
}

pub fn in_al_dx(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.registers.dx();
    let val = machine.in_port(port);
    cpu.registers.set_al(val);
}

pub fn out_dx_al(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.registers.dx();
    let val = cpu.registers.al();
    machine.out_port(port, val);
}

pub fn in_al_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    let val = machine.in_port(port);
    cpu.registers.set_al(val);
}

pub fn clc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f &= !flags::CF;
    cpu.registers.set_flags(f);
}

pub fn stc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f |= flags::CF;
    cpu.registers.set_flags(f);
}

pub fn cbw(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let ax = cpu.registers.ax() as i16 as i32 as u32;
        cpu.registers.set_eax(ax);
    } else {
        let al = cpu.registers.al() as i8 as i16 as u16;
        cpu.registers.set_ax(al);
    }
}

pub fn cwd(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let eax = cpu.registers.eax() as i32;
        cpu.registers.set_edx(if eax < 0 { 0xFFFF_FFFF } else { 0 });
    } else {
        let ax = cpu.registers.ax() as i16;
        cpu.registers.set_dx(if ax < 0 { 0xFFFF } else { 0 });
    }
}

pub fn wait(_cpu: &mut X86Cpu, _machine: &mut dyn Machine) {}

pub fn sahf(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    const MASK: u16 = 0x00D5;
    let ah = cpu.registers.ah() as u16;
    let mut flags = cpu.registers.flags();
    flags = (flags & !MASK) | (ah & MASK);
    cpu.registers.set_flags(flags);
}

pub fn lahf(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let ah = (cpu.registers.flags() & 0x00D5) as u8 | 0x02;
    cpu.registers.set_ah(ah);
}

pub fn into(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if flags::test_of(cpu.registers.flags()) {
        software_interrupt(cpu, machine, 4);
    }
}

pub fn out_imm8_al(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    machine.out_port(port, cpu.registers.al());
}

pub fn in_ax_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    read_port_ax(cpu, machine, port);
}

pub fn out_imm8_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let port = cpu.fetch_u8(machine) as u16;
    write_port_ax(cpu, machine, port);
}

pub fn in_ax_dx(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    read_port_ax(cpu, machine, cpu.registers.dx());
}

pub fn out_dx_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    write_port_ax(cpu, machine, cpu.registers.dx());
}

fn read_port_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine, port: u16) {
    if cpu.prefixes.has_operand_size {
        let mut v = 0u32;
        for i in 0..4 {
            v |= (machine.in_port(port.wrapping_add(i)) as u32) << (8 * i);
        }
        cpu.registers.set_eax(v);
    } else {
        let lo = machine.in_port(port) as u16;
        let hi = machine.in_port(port.wrapping_add(1)) as u16;
        cpu.registers.set_ax(lo | (hi << 8));
    }
}

fn write_port_ax(cpu: &mut X86Cpu, machine: &mut dyn Machine, port: u16) {
    if cpu.prefixes.has_operand_size {
        let v = cpu.registers.eax();
        for i in 0..4 {
            machine.out_port(port.wrapping_add(i), (v >> (8 * i)) as u8);
        }
    } else {
        let v = cpu.registers.ax();
        machine.out_port(port, v as u8);
        machine.out_port(port.wrapping_add(1), (v >> 8) as u8);
    }
}

pub fn arpl(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let src_rpl = cpu.read_reg16(modrm.reg_field) & 0x03;
    let (dst_old, addr) = if modrm.is_register_mode() {
        (cpu.read_reg16(modrm.rm_field), None)
    } else {
        let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
        (machine.read_mem_u16(addr), Some(addr))
    };
    let raised = src_rpl > (dst_old & 0x03);
    if raised {
        let dst_new = (dst_old & 0xFFFC) | src_rpl;
        if let Some(addr) = addr {
            machine.write_mem_u16(addr, dst_new);
        } else {
            cpu.write_reg16(modrm.rm_field, dst_new);
        }
    }
    let mut f = cpu.registers.flags();
    if raised {
        f |= flags::ZF;
    } else {
        f &= !flags::ZF;
    }
    cpu.registers.set_flags(f);
}

pub fn imul_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine, imm8: bool) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg32(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u32(addr)
        };
        let imm = if imm8 {
            cpu.fetch_u8(machine) as i8 as i32 as u32
        } else {
            cpu.fetch_u32(machine)
        };
        let full = (src as i32 as i64) * (imm as i32 as i64);
        let result = full as u32;
        cpu.write_reg32(modrm.reg_field, result);
        set_mul_overflow(cpu, full != result as i32 as i64);
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        let imm = if imm8 {
            cpu.fetch_u8(machine) as i8 as i16 as u16
        } else {
            cpu.fetch_u16(machine)
        };
        let full = (src as i16 as i32) * (imm as i16 as i32);
        let result = full as u16;
        cpu.write_reg16(modrm.reg_field, result);
        set_mul_overflow(cpu, full != result as i16 as i32);
    }
}

fn set_mul_overflow(cpu: &mut X86Cpu, overflow: bool) {
    let mut f = cpu.registers.flags();
    if overflow {
        f |= flags::CF | flags::OF;
    } else {
        f &= !(flags::CF | flags::OF);
    }
    cpu.registers.set_flags(f);
}

pub fn aam(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let base = cpu.fetch_u8(machine);
    if base == 0 {
        software_interrupt(cpu, machine, 0);
        return;
    }
    let al = cpu.registers.al();
    cpu.registers.set_ah(al / base);
    let new_al = al % base;
    cpu.registers.set_al(new_al);
    let f = flags::compute_flags_u8(cpu.registers.flags(), new_al, false, false, false);
    let old_cf = cpu.registers.flags() & flags::CF;
    let old_of = cpu.registers.flags() & flags::OF;
    let old_af = cpu.registers.flags() & flags::AF;
    cpu.registers
        .set_flags((f & !(flags::CF | flags::OF | flags::AF)) | old_cf | old_of | old_af);
}

pub fn aad(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let base = cpu.fetch_u8(machine);
    let al = cpu.registers.ah().wrapping_mul(base).wrapping_add(cpu.registers.al());
    cpu.registers.set_ax(al as u16);
    let f = flags::compute_flags_u8(cpu.registers.flags(), al, false, false, false);
    let kept = cpu.registers.flags() & (flags::CF | flags::OF | flags::AF);
    cpu.registers.set_flags((f & !(flags::CF | flags::OF | flags::AF)) | kept);
}

pub fn salc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    cpu.registers
        .set_al(if flags::test_cf(cpu.registers.flags()) { 0xFF } else { 0 });
}

pub fn xlat(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let seg = cpu.prefixes.segment_override.unwrap_or(cpu.registers.ds());
    let offset = cpu.registers.bx().wrapping_add(cpu.registers.al() as u16);
    cpu.registers
        .set_al(machine.read_mem_u8(cpu.phys_addr(seg, offset)));
}

pub fn pop_fs(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = crate::instructions::stack::pop16(cpu, machine);
    cpu.registers.set_fs(v);
}

pub fn group_0f01(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.reg_field <= 3 && modrm.is_register_mode() {
        log::error!("0F 01 /{} with mod=11 is undefined", modrm.reg_field);
        cpu.halted = true;
        return;
    }
    match modrm.reg_field {
        0 | 1 => {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, 0xFFFF);
            machine.write_mem_u32(addr.wrapping_add(2), 0);
        }
        2 => {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            cpu.gdtr_limit = machine.read_mem_u16(addr);
            cpu.gdtr_base = machine.read_mem_u32(addr.wrapping_add(2)) & 0x00FF_FFFF;
        }
        3 => {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            cpu.idtr_limit = machine.read_mem_u16(addr);
            cpu.idtr_base = machine.read_mem_u32(addr.wrapping_add(2)) & 0x00FF_FFFF;
        }
        4 => {
            let msw = (cpu.cr0 & 0xFFFF) as u16;
            if modrm.is_register_mode() {
                cpu.write_reg16(modrm.rm_field, msw);
            } else {
                let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
                machine.write_mem_u16(addr, msw);
            }
        }
        _ => {
            log::warn!("Unhandled 0F 01 /{}", modrm.reg_field);
            if !modrm.is_register_mode() {
                let _ = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            }
        }
    }
}

pub fn mov_r32_cr(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let value = match modrm.reg_field {
        0 => cpu.cr0,
        _ => {
            log::error!("MOV r32, CR{}", modrm.reg_field);
            cpu.halted = true;
            return;
        }
    };
    cpu.write_reg32(modrm.rm_field, value);
}

pub fn mov_cr_r32(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    let value = cpu.read_reg32(modrm.rm_field);
    match modrm.reg_field {
        0 => cpu.cr0 = value,
        _ => {
            log::error!("MOV CR{}, r32", modrm.reg_field);
            cpu.halted = true;
        }
    }
}

pub fn movzx(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = crate::modrm::ModRm::from_byte(cpu.fetch_u8(machine));
    if cpu.prefixes.has_operand_size {
        let src = if modrm.is_register_mode() {
            cpu.read_reg16(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u16(addr)
        };
        cpu.write_reg32(modrm.reg_field, src as u32);
    } else {
        let src = if modrm.is_register_mode() {
            cpu.read_reg8(modrm.rm_field)
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.read_mem_u8(addr)
        };
        cpu.write_reg16(modrm.reg_field, src as u16);
    }
}

pub fn int1(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    log::error!(
        "INT1 (0xF1) not implemented at CS:IP = {:04X}:{:04X}",
        cpu.registers.cs(),
        cpu.registers.ip().wrapping_sub(1)
    );
    cpu.halted = true;
}

pub fn cmc(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f ^= flags::CF;
    cpu.registers.set_flags(f);
}

pub fn cld(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f &= !flags::DF;
    cpu.registers.set_flags(f);
}

pub fn std(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f |= flags::DF;
    cpu.registers.set_flags(f);
}

pub fn cli(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f &= !flags::IF;
    cpu.registers.set_flags(f);
}

pub fn sti(cpu: &mut X86Cpu, _machine: &mut dyn Machine) {
    let mut f = cpu.registers.flags();
    f |= flags::IF;
    cpu.registers.set_flags(f);
}