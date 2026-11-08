// Ver: 1 File: crate/x86/src/fpu/esc.rs
//! Диспетчер однобайтовых FPU-эскейпов D8..DF.
//!
//! Подфаза 3.3: реализованы FINIT/FNINIT, FLDCW/FNSTCW, FNSTSW.
//! Остальные группы — честная остановка с диагностикой до своих подфаз.
//!
//! Про #NM: все D8..DF блокируются при CR0.EM || CR0.TS.
//! Спец-правило WAIT (em || (ts && mp)) относится к опкоду 0x9B, который
//! обрабатывается в executor.rs и сюда не попадает.

use bus::Machine;

use crate::cpu::X86Cpu;
use crate::fpu::control;
use crate::modrm::ModRm;

// Биты CR0, релевантные FPU.
pub const CR0_MP: u32 = 1 << 1; // Monitor Coprocessor
pub const CR0_EM: u32 = 1 << 2; // Emulate
pub const CR0_TS: u32 = 1 << 3; // Task Switched

/// Единая точка входа для опкодов D8..DF.
pub fn execute_esc(cpu: &mut X86Cpu, machine: &mut dyn Machine, opcode: u8) {
    let em = cpu.cr0 & CR0_EM != 0;
    let ts = cpu.cr0 & CR0_TS != 0;

    // Все ESC-инструкции: #NM при EM либо TS.
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

    let modrm = ModRm::from_byte(cpu.fetch_u8(machine));
    let addr = if !modrm.is_register_mode() {
        Some(modrm.resolve_address(
            cpu,
            machine,
            cpu.prefixes.has_address_size,
        ))
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
}

fn not_impl(cpu: &mut X86Cpu, opcode: u8, modrm: &ModRm) {
    let bytes = &cpu.instruction_bytes;
    let bytes_hex = bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
    
    log::error!(
        "=== FPU INSTRUCTION NOT IMPLEMENTED ===\n\
        Opcode: {:02X} | Full Bytes: [ {} ]\n\
        ModRM: mod={} reg={} rm={} (is_reg_mode={})\n\
        Location: CS:IP = {:04X}:{:04X}\n\
        FPU State: TOP={} | Status={:04X} | Control={:04X}\n\
        ST(0)={:} | ST(1)={:} | ST(2)={:}\n\
        =======================================",
        opcode, bytes_hex,
        modrm.mod_field, modrm.reg_field, modrm.rm_field, modrm.is_register_mode(),
        cpu.instruction_start_cs, cpu.instruction_start_ip,
        cpu.fpu.top(), cpu.fpu.status, cpu.fpu.control,
        cpu.fpu.st(0).hex(), cpu.fpu.st(1).hex(), cpu.fpu.st(2).hex()
    );
    cpu.halted = true;
}

// D8: FADD/FMUL/FCOM/FCOMP/FSUB/FSUBR/FDIV/FDIVR (mem/reg) — Фаза 3.5
fn dispatch_d8(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    not_impl(cpu, 0xD8, modrm);
}

// D9: FLDCW / FNSTCW / FNINIT / FLD-константы / FST/FSTP m32/m64/m80 / FSQRT ...
fn dispatch_d9(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: &ModRm, addr: Option<u32>) {
    match modrm.reg_field {
        5 => {
            // FLDCW m16 — только memory mode.
            match addr {
                Some(a) => control::fldcw(cpu, machine, a),
                None => not_impl(cpu, 0xD9, modrm),
            }
        }
        7 => {
            if modrm.is_register_mode() {
                // FNINIT: D9 E0 (mod=11, rm=000).
                if modrm.rm_field == 0 {
                    control::fninit(cpu);
                } else {
                    not_impl(cpu, 0xD9, modrm);
                }
            } else {
                // FNSTCW m16.
                match addr {
                    Some(a) => control::fnstcw(cpu, machine, a),
                    None => not_impl(cpu, 0xD9, modrm),
                }
            }
        }
        _ => not_impl(cpu, 0xD9, modrm),
    }
}

// DA: FIADD/FIMUL/FICOM/FICOMP/FISUB/FISUBR/FIDIV/FIDIVR — Фаза 3.5
fn dispatch_da(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    not_impl(cpu, 0xDA, modrm);
}

// DB: FILD/FIST/FLD m80/FST m80/FLDENV/FXAM/FNCLEX/FINIT + FCMOVcc
fn dispatch_db(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    match modrm.reg_field {
        7 => {
            // FINIT: DB E0 (mod=11, rm=000).
            if modrm.is_register_mode() && modrm.rm_field == 0 {
                control::finit(cpu);
            } else {
                not_impl(cpu, 0xDB, modrm);
            }
        }
        _ => not_impl(cpu, 0xDB, modrm),
    }
}

// DC: FADD/FMUL/FCOM/FCOMP/FSUB/FSUBR/FDIV/FDIVR (ST0,STi) — Фаза 3.5
fn dispatch_dc(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    not_impl(cpu, 0xDC, modrm);
}

// DD: FLD STi / FST/FSTP STi / FRSTOR / FSAVE / FST m32/m64 / FFREE / FXCH / ...
fn dispatch_dd(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    not_impl(cpu, 0xDD, modrm);
}

// DE: FIADD/FIMUL/FICOM/FICOMP/FISUB/FISUBR/FIDIV/FIDIVR (ST0,STi) — Фаза 3.5
fn dispatch_de(cpu: &mut X86Cpu, _machine: &mut dyn Machine, modrm: &ModRm, _addr: Option<u32>) {
    not_impl(cpu, 0xDE, modrm);
}

// DF: FCOMI / FST m64int / FISTTP / FBLD / FBSTP / FNSTSW
fn dispatch_df(cpu: &mut X86Cpu, machine: &mut dyn Machine, modrm: &ModRm, addr: Option<u32>) {
    match modrm.reg_field {
        7 => {
            if modrm.is_register_mode() {
                // FNSTSW AX: DF E0 (mod=11, rm=000).
                if modrm.rm_field == 0 {
                    control::fnstsw_ax(cpu);
                } else {
                    not_impl(cpu, 0xDF, modrm);
                }
            } else {
                // FNSTSW m16.
                match addr {
                    Some(a) => control::fnstsw_m16(cpu, machine, a),
                    None => not_impl(cpu, 0xDF, modrm),
                }
            }
        }
        _ => not_impl(cpu, 0xDF, modrm),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpu::state::CTRL_DEFAULT;
    use crate::fpu::F80;
    use bus::Motherboard;

    /// Детерминированный CPU: CS=0, EIP=0, чтобы execute_esc читал ModRM
    /// из физического адреса 0, куда тесты пишут байты.
    fn fresh_cpu() -> X86Cpu {
        let mut cpu = X86Cpu::new();
        cpu.registers.set_cs(0);
        cpu.registers.set_eip(0);
        cpu
    }

    // === Блокировка по CR0 ===

    #[test]
    fn test_em_blocks_esc() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.cr0 = CR0_EM;
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(cpu.halted, "CR0.EM must block FPU ESC");
        // Блокировка до чтения ModRM -> EIP не сдвинут.
        assert_eq!(cpu.registers.eip(), 0);
        // Состояние FPU не тронуто.
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
        // Регрессия бага 3.2: D9 (FNINIT) при TS=1 обязан блокироваться,
        // как и все остальные ESC.
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.cr0 = CR0_TS;
        mb.write_mem_u8(0, 0xE0); // D9 E0 = FNINIT
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(cpu.halted, "D9 must block on CR0.TS");
        assert_eq!(cpu.registers.eip(), 0, "ModRM не должен читаться");
        // Блокировка НЕ сбрасывает FPU: control остаётся значением по умолчанию.
        assert_eq!(cpu.fpu.control, CTRL_DEFAULT);
    }

    #[test]
    fn test_unimplemented_group_halts() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        // D8 /0 mod=11 rm=0 -> FADD ST(0),ST(0), не реализовано.
        mb.write_mem_u8(0, 0xC0);
        execute_esc(&mut cpu, &mut mb, 0xD8);
        assert!(cpu.halted);
        assert_eq!(cpu.registers.eip(), 1, "ModRM должен быть прочитан");
    }


    // === Memory-mode сквозные пути ===
    //
    // mod=01, rm=000 ([BX+SI]+disp8) при DS=BX=SI=0, disp8=0x10
    // -> операнд по физическому адресу 0x10.

    #[test]
    fn test_fldcw_memory() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.registers.set_bx(0);
        cpu.registers.set_si(0);
        mb.write_mem_u16(0x10, 0x1234);
        mb.write_mem_u8(0, 0x68); // D9 /5 mod=01 rm=000
        mb.write_mem_u8(1, 0x10); // disp8
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
        mb.write_mem_u8(0, 0x78); // D9 /7 mod=01 rm=000
        mb.write_mem_u8(1, 0x10); // disp8
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted);
        assert_eq!(mb.read_mem_u16(0x10), 0xABCD);
        assert_eq!(cpu.registers.eip(), 2);
    }

    #[test]
    fn test_fnstsw_m16_memory() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.registers.set_bx(0);
        cpu.registers.set_si(0);
        cpu.fpu.status = 0x1234;
        mb.write_mem_u8(0, 0x78); // DF /7 mod=01 rm=000
        mb.write_mem_u8(1, 0x10); // disp8
        execute_esc(&mut cpu, &mut mb, 0xDF);
        assert!(!cpu.halted);
        assert_eq!(mb.read_mem_u16(0x10), 0x1234);
        assert_eq!(cpu.registers.eip(), 2);
    }

        #[test]
    fn test_fninit_d9_e0() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(false)).unwrap();
        cpu.fpu.status = 0xFFFF;
        cpu.fpu.control = 0;
        mb.write_mem_u8(0, 0xF8); // D9 F8 = FNINIT (mod=11, reg=7, rm=0)
        execute_esc(&mut cpu, &mut mb, 0xD9);
        assert!(!cpu.halted, "FNINIT должен выполниться");
        assert_eq!(cpu.fpu.status, 0);
        assert_eq!(cpu.fpu.control, CTRL_DEFAULT);
        assert_eq!(cpu.fpu.top(), 0);
        assert_eq!(cpu.registers.eip(), 1);
    }

    #[test]
    fn test_finint_db_e0() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.push(F80::one(false)).unwrap();
        cpu.fpu.status = 0xFFFF;
        cpu.fpu.control = 0;
        mb.write_mem_u8(0, 0xF8); // DB F8 = FINIT (mod=11, reg=7, rm=0)
        execute_esc(&mut cpu, &mut mb, 0xDB);
        assert!(!cpu.halted, "FINIT должен выполниться");
        assert_eq!(cpu.fpu.status, 0);
        assert_eq!(cpu.fpu.control, CTRL_DEFAULT);
        assert_eq!(cpu.fpu.top(), 0);
    }

    #[test]
    fn test_fnstsw_ax_df_e0() {
        let mut cpu = fresh_cpu();
        let mut mb = Motherboard::new(64);
        cpu.fpu.status = 0xABCD;
        mb.write_mem_u8(0, 0xF8); // DF F8 = FNSTSW AX (mod=11, reg=7, rm=0)
        execute_esc(&mut cpu, &mut mb, 0xDF);
        assert!(!cpu.halted);
        assert_eq!(cpu.registers.ax(), 0xABCD);
        assert_eq!(cpu.registers.eip(), 1);
    }
}