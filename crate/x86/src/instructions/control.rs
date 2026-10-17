// Ver: 1 File: ./crate/x86/src/instructions/control.rs
use crate::cpu::X86Cpu;
use crate::flags;
use crate::instructions::stack;
use crate::modrm::ModRm;
use bus::Machine;

pub fn cond(cpu: &X86Cpu, opcode: u8) -> bool {
    let f = cpu.registers.flags();
    let cf = flags::test_cf(f);
    let zf = flags::test_zf(f);
    let sf = flags::test_sf(f);
    let of = flags::test_of(f);
    let pf = flags::test_pf(f);
    match opcode & 0x0F {
        0x0 => of,
        0x1 => !of,
        0x2 => cf,
        0x3 => !cf,
        0x4 => zf,
        0x5 => !zf,
        0x6 => cf || zf,
        0x7 => !cf && !zf,
        0x8 => sf,
        0x9 => !sf,
        0xA => pf,
        0xB => !pf,
        0xC => sf != of,
        0xD => sf == of,
        0xE => zf || sf != of,
        0xF => !zf && sf == of,
        _ => unreachable!(),
    }
}

pub fn jcc_rel8(cpu: &mut X86Cpu, machine: &mut dyn Machine, opcode: u8) {
    let rel = cpu.fetch_u8(machine) as i8;
    if cond(cpu, opcode) {
        let ip = cpu.registers.ip().wrapping_add(rel as u16);
        cpu.registers.set_ip(ip);
    }
}

pub fn jcc_rel16(cpu: &mut X86Cpu, machine: &mut dyn Machine, opcode: u8) {
    let rel = if cpu.prefixes.has_operand_size {
        cpu.fetch_u32(machine) as i32
    } else {
        cpu.fetch_u16(machine) as i16 as i32
    };
    if cond(cpu, opcode) {
        let ip = (cpu.registers.ip() as i32).wrapping_add(rel) as u16;
        cpu.registers.set_ip(ip);
    }
}

pub fn loopnz(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    loop_cc(cpu, machine, Some(false));
}

pub fn loopz(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    loop_cc(cpu, machine, Some(true));
}

pub fn loop_rel(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    loop_cc(cpu, machine, None);
}

fn loop_cc(cpu: &mut X86Cpu, machine: &mut dyn Machine, zf_must: Option<bool>) {
    let rel = cpu.fetch_u8(machine) as i8;
    let cx = if cpu.prefixes.has_address_size {
        let n = cpu.registers.ecx().wrapping_sub(1);
        cpu.registers.set_ecx(n);
        n
    } else {
        let n = cpu.registers.cx().wrapping_sub(1) as u32;
        cpu.registers.set_cx(n as u16);
        n
    };
    let zf_ok = match zf_must {
        Some(true) => flags::test_zf(cpu.registers.flags()),
        Some(false) => !flags::test_zf(cpu.registers.flags()),
        None => true,
    };
    if cx != 0 && zf_ok {
        let ip = cpu.registers.ip().wrapping_add(rel as u16);
        cpu.registers.set_ip(ip);
    }
}

pub fn jcxz(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let rel = cpu.fetch_u8(machine) as i8;
    let zero = if cpu.prefixes.has_address_size {
        cpu.registers.ecx() == 0
    } else {
        cpu.registers.cx() == 0
    };
    if zero {
        let ip = cpu.registers.ip().wrapping_add(rel as u16);
        cpu.registers.set_ip(ip);
    }
}

pub fn call_rel(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let rel = cpu.fetch_u32(machine) as i32;
        let ret = cpu.registers.ip();
        stack::push32(cpu, machine, ret as u32);
        cpu.registers.set_ip((ret as i32).wrapping_add(rel) as u16);
    } else {
        let rel = cpu.fetch_u16(machine) as i16;
        let ret = cpu.registers.ip();
        stack::push16(cpu, machine, ret);
        cpu.registers
            .set_ip((ret as i32).wrapping_add(rel as i32) as u16);
    }
}

pub fn jmp_rel(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    if cpu.prefixes.has_operand_size {
        let rel = cpu.fetch_u32(machine) as i32;
        let ip = (cpu.registers.ip() as i32).wrapping_add(rel) as u16;
        cpu.registers.set_ip(ip);
    } else {
        let rel = cpu.fetch_u16(machine) as i16;
        let ip = (cpu.registers.ip() as i32).wrapping_add(rel as i32) as u16;
        cpu.registers.set_ip(ip);
    }
}

pub fn jmp_far(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let ip = cpu.fetch_u16(machine);
    let cs = cpu.fetch_u16(machine);
    cpu.registers.set_cs(cs);
    cpu.registers.set_ip(ip);
}

pub fn jmp_rel8(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let rel = cpu.fetch_u8(machine) as i8;
    let ip = cpu.registers.ip().wrapping_add(rel as u16);
    cpu.registers.set_ip(ip);
}

pub fn call_far(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let ip = cpu.fetch_u16(machine);
    let cs = cpu.fetch_u16(machine);
    let ret_cs = cpu.registers.cs();
    let ret_ip = cpu.registers.ip();
    stack::push16(cpu, machine, ret_cs);
    stack::push16(cpu, machine, ret_ip);
    cpu.registers.set_cs(cs);
    cpu.registers.set_ip(ip);
}

pub fn ret_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    ret_near(cpu, machine, imm);
}

pub fn ret_near0(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    ret_near(cpu, machine, 0);
}

fn ret_near(cpu: &mut X86Cpu, machine: &mut dyn Machine, extra: u16) {
    if cpu.prefixes.has_operand_size {
        let ip = stack::pop32(cpu, machine) as u16;
        cpu.registers.set_ip(ip);
        cpu.registers
            .set_sp(cpu.registers.sp().wrapping_add(extra));
    } else {
        let ip = stack::pop16(cpu, machine);
        cpu.registers.set_ip(ip);
        cpu.registers
            .set_sp(cpu.registers.sp().wrapping_add(extra));
    }
}

pub fn retf_imm(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let imm = cpu.fetch_u16(machine);
    ret_far(cpu, machine, imm);
}

pub fn retf(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    ret_far(cpu, machine, 0);
}

fn ret_far(cpu: &mut X86Cpu, machine: &mut dyn Machine, extra: u16) {
    let ip = stack::pop16(cpu, machine);
    let cs = stack::pop16(cpu, machine);
    cpu.registers.set_ip(ip);
    cpu.registers.set_cs(cs);
    cpu.registers
        .set_sp(cpu.registers.sp().wrapping_add(extra));
}

pub fn enter(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let alloc = cpu.fetch_u16(machine);
    let level = cpu.fetch_u8(machine) & 0x1F;
    let frame = cpu.registers.bp();
    stack::push16(cpu, machine, frame);
    let frame_sp = cpu.registers.sp();
    if level > 0 {
        for i in 1..level {
            let bp = cpu.registers.bp().wrapping_sub((i as u16) * 2);
            let val = machine.read_mem_u16(cpu.phys_addr(cpu.registers.ss(), bp));
            stack::push16(cpu, machine, val);
        }
        stack::push16(cpu, machine, frame_sp);
    }
    cpu.registers.set_bp(frame_sp);
    cpu.registers.set_sp(frame_sp.wrapping_sub(alloc));
}

pub fn leave(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    cpu.registers.set_sp(cpu.registers.bp());
    let bp = stack::pop16(cpu, machine);
    cpu.registers.set_bp(bp);
}

pub fn bound(cpu: &mut X86Cpu, machine: &mut dyn Machine) {
    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    if modrm.is_register_mode() {
        log::error!("BOUND with register operand");
        cpu.halted = true;
        return;
    }
    let addr = modrm.resolve_address(cpu, machine, cpu.prefixes.has_address_size);
    if cpu.prefixes.has_operand_size {
        let index = cpu.read_reg32(modrm.reg_field) as i32;
        let lower = machine.read_mem_u32(addr) as i32;
        let upper = machine.read_mem_u32(addr.wrapping_add(4)) as i32;
        if index < lower || index > upper {
            crate::instructions::system::software_interrupt(cpu, machine, 5);
        }
    } else {
        let index = cpu.read_reg16(modrm.reg_field) as i16;
        let lower = machine.read_mem_u16(addr) as i16;
        let upper = machine.read_mem_u16(addr.wrapping_add(2)) as i16;
        if index < lower || index > upper {
            crate::instructions::system::software_interrupt(cpu, machine, 5);
        }
    }
}
