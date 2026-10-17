// Ver: 1 File: ./crate/x86/src/instructions/stack.rs
use crate::cpu::X86Cpu;
use crate::modrm::ModRm;
use bus::Machine;

#[inline]
fn ss_addr(cpu: &X86Cpu, sp: u16) -> u32 {
    cpu.phys_addr(cpu.registers.ss(), sp)
}

pub fn push16(cpu: &mut X86Cpu, machine: &mut dyn Machine, val: u16) {
    let sp = cpu.registers.sp().wrapping_sub(2);
    machine.write_mem_u16(ss_addr(cpu, sp), val);
    cpu.registers.set_sp(sp);
}

pub fn pop16(cpu: &mut X86Cpu, machine: &mut dyn Machine) -> u16 {
    let sp = cpu.registers.sp();
    let val = machine.read_mem_u16(ss_addr(cpu, sp));
    cpu.registers.set_sp(sp.wrapping_add(2));
    val
}

pub fn push32(cpu: &mut X86Cpu, machine: &mut dyn Machine, val: u32) {
    let esp = cpu.registers.esp().wrapping_sub(4);
    machine.write_mem_u32(ss_addr(cpu, esp as u16), val);
    cpu.registers.set_esp(esp);
}

pub fn pop32(cpu: &mut X86Cpu, machine: &mut dyn Machine) -> u32 {
    let esp = cpu.registers.esp();
    let val = machine.read_mem_u32(ss_addr(cpu, esp as u16));
    cpu.registers.set_esp(esp.wrapping_add(4));
    val
}

pub fn push_es(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = cpu.registers.es();
    push16(cpu, machine, v);
}

pub fn pop_es(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = pop16(cpu, machine);
    cpu.registers.set_es(v);
}

pub fn push_cs(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = cpu.registers.cs();
    push16(cpu, machine, v);
}

pub fn push_ss(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = cpu.registers.ss();
    push16(cpu, machine, v);
}

pub fn pop_ss(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = pop16(cpu, machine);
    cpu.registers.set_ss(v);
    cpu.inhibit_interrupts = true;
}

pub fn push_ds(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = cpu.registers.ds();
    push16(cpu, machine, v);
}

pub fn pop_ds(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let v = pop16(cpu, machine);
    cpu.registers.set_ds(v);
}

pub fn push_reg(cpu: &mut X86Cpu, machine: &mut dyn Machine, reg: u8) {
    if cpu.prefixes.has_operand_size {
        let val = cpu.read_reg32(reg);
        push32(cpu, machine, val);
    } else if reg == 4 {
        let val = cpu.registers.sp();
        push16(cpu, machine, val);
    } else {
        let val = cpu.read_reg16(reg);
        push16(cpu, machine, val);
    }
}

pub fn pop_reg(cpu: &mut X86Cpu, machine: &mut dyn Machine, reg: u8) {
    if cpu.prefixes.has_operand_size {
        let val = pop32(cpu, machine);
        cpu.write_reg32(reg, val);
    } else {
        let val = pop16(cpu, machine);
        cpu.write_reg16(reg, val);
    }
}

pub fn pusha(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let temp = cpu.registers.esp();
        let regs = [
            cpu.registers.eax(),
            cpu.registers.ecx(),
            cpu.registers.edx(),
            cpu.registers.ebx(),
            temp,
            cpu.registers.ebp(),
            cpu.registers.esi(),
            cpu.registers.edi(),
        ];
        for val in regs {
            push32(cpu, machine, val);
        }
    } else {
        let temp = cpu.registers.sp();
        let regs = [
            cpu.registers.ax(),
            cpu.registers.cx(),
            cpu.registers.dx(),
            cpu.registers.bx(),
            temp,
            cpu.registers.bp(),
            cpu.registers.si(),
            cpu.registers.di(),
        ];
        for val in regs {
            push16(cpu, machine, val);
        }
    }
}

pub fn popa(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let edi = pop32(cpu, machine);
        let esi = pop32(cpu, machine);
        let ebp = pop32(cpu, machine);
        let _esp = pop32(cpu, machine);
        let ebx = pop32(cpu, machine);
        let edx = pop32(cpu, machine);
        let ecx = pop32(cpu, machine);
        let eax = pop32(cpu, machine);
        cpu.registers.set_edi(edi);
        cpu.registers.set_esi(esi);
        cpu.registers.set_ebp(ebp);
        cpu.registers.set_ebx(ebx);
        cpu.registers.set_edx(edx);
        cpu.registers.set_ecx(ecx);
        cpu.registers.set_eax(eax);
    } else {
        let di = pop16(cpu, machine);
        let si = pop16(cpu, machine);
        let bp = pop16(cpu, machine);
        let _sp = pop16(cpu, machine);
        let bx = pop16(cpu, machine);
        let dx = pop16(cpu, machine);
        let cx = pop16(cpu, machine);
        let ax = pop16(cpu, machine);
        cpu.registers.set_di(di);
        cpu.registers.set_si(si);
        cpu.registers.set_bp(bp);
        cpu.registers.set_bx(bx);
        cpu.registers.set_dx(dx);
        cpu.registers.set_cx(cx);
        cpu.registers.set_ax(ax);
    }
}

pub fn push_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let imm = cpu.fetch_u32(machine);
        push32(cpu, machine, imm);
    } else {
        let imm = cpu.fetch_u16(machine);
        push16(cpu, machine, imm);
    }
}

pub fn push_imm8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u8(machine) as i8;
    if cpu.prefixes.has_operand_size {
        push32(cpu, machine, imm as u32);
    } else {
        push16(cpu, machine, imm as u16);
    }
}

pub fn pop_rm(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.reg_field != 0 {
        log::error!("POP r/m: reg field {:#x} is not 0", modrm.reg_field);
        cpu.halted = true;
        return;
    }
    if cpu.prefixes.has_operand_size {
        let val = pop32(cpu, machine);
        if modrm.is_register_mode() {
            cpu.write_reg32(modrm.rm_field, val);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u32(addr, val);
        }
    } else {
        let val = pop16(cpu, machine);
        if modrm.is_register_mode() {
            cpu.write_reg16(modrm.rm_field, val);
        } else {
            let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
            machine.write_mem_u16(addr, val);
        }
    }
}

pub fn pushf(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let val = cpu.registers.eflags() & 0x00FC_FFFF;
        push32(cpu, machine, val);
    } else {
        let val = cpu.registers.flags();
        push16(cpu, machine, val);
    }
}

pub fn popf(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let val = pop32(cpu, machine);
        let kept = cpu.registers.eflags() & 0x0002_0000;
        cpu.registers.set_eflags((val & 0x0024_7FD5) | kept | 0x2);
    } else {
        let val = pop16(cpu, machine);
        cpu.registers.set_flags(val);
    }
}
