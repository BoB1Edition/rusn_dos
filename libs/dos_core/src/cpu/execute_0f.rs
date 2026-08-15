// Ver: 1 File: ./libs/dos_core/src/cpu/execute_0f.rs
use crate::{
    DosMachine,
    instructions::{control, control32, extended, extended32, stack},
    modrm::ModRm,
};

pub(crate) fn execute_0f(machine: &mut DosMachine, opcode: u8 /* ,debug: Option<&DebugLog>*/) {
    let mut full_bytes = Vec::new();
    if machine.has_operand_size_prefix {
        full_bytes.push(0x66);
    }
    if machine.has_address_size_prefix {
        full_bytes.push(0x67);
    }
    if let Some(oos) = machine.opcode_override_segment {
        full_bytes.push(oos);
    }
    full_bytes.push(0x0F);
    full_bytes.push(opcode);

    match opcode {
        0x01 => {
            let modrm_byte = machine.read_instr_u8(machine.registers.ip());
            machine.registers.step(None); // шаг на ModR/M
            let modrm = ModRm::from_byte(modrm_byte);

            // Для reg_field 0-3 модификатор mod=11 запрещён
            if modrm.reg_field <= 3 && modrm.is_register_mode() {
                log::error!("0F 01 /{} with mod=11 is undefined", modrm.reg_field);
                machine.halted = true;
                return;
            }

            let mut bytes = full_bytes.clone();
            bytes.push(modrm_byte);

            match modrm.reg_field {
                0 => {
                    // SGDT — сохранить GDTR
                    let addr = modrm
                        .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
                        .unwrap();
                    // В реальном режиме GDTR: база=0, лимит=0xFFFF
                    machine.write_phys_u16(addr, 0xFFFF); // лимит
                    machine.write_phys_u32(addr.wrapping_add(2), 0); // база (24 бита, но пишем 32)
                }
                1 => {
                    // SIDT — сохранить IDTR
                    let addr = modrm
                        .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
                        .unwrap();
                    machine.write_phys_u16(addr, 0xFFFF);
                    machine.write_phys_u32(addr.wrapping_add(2), 0);
                }
                2 => {
                    // LGDT — загрузить GDTR
                    let addr = modrm
                        .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
                        .unwrap();
                    let limit = machine.read_phys_u16(addr);
                    let base = machine.read_phys_u32(addr.wrapping_add(2)) & 0x00FF_FFFF; // только 24 бита
                    // Сохраняем в структуру машины (можно добавить поля gdtr_base, gdtr_limit)
                    machine.gdtr_limit = limit;
                    machine.gdtr_base = base;
                    log::info!("LGDT: base={:06X}, limit={:04X}", base, limit);
                }
                3 => {
                    // LIDT — загрузить IDTR
                    let addr = modrm
                        .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
                        .unwrap();
                    let limit = machine.read_phys_u16(addr);
                    let base = machine.read_phys_u32(addr.wrapping_add(2)) & 0x00FF_FFFF;
                    machine.idtr_limit = limit;
                    machine.idtr_base = base;
                    log::info!("LIDT: base={:06X}, limit={:04X}", base, limit);
                }
                4 => {
                    // SMSW
                    control::smsw(machine, &full_bytes);
                }
                _ => {
                    log::warn!("Unhandled 0F 01 /{}", modrm.reg_field);
                    // Пропускаем операнд (адрес в памяти), если mod != 11
                    if !modrm.is_register_mode() {
                        modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes);
                    }
                }
            }

            // Логирование
            machine
                .log_instruction(
                    [
                        machine.registers.cs(),
                        machine.registers.ip() - bytes.len() as u16,
                    ],
                    &bytes,
                )
                .ok();
        }
        0xA1 => {
            stack::pop_fs(machine);
            machine
                .log_instruction(
                    [
                        machine.registers.cs(),
                        machine.registers.ip() - full_bytes.len() as u16,
                    ],
                    &full_bytes,
                )
                .ok();
        }
        0x20 => extended::mov_reg32_crn(machine, &full_bytes),
        0x22 => extended::mov_crn_reg32(machine, &full_bytes),
        0x82 => {
            if machine.has_operand_size_prefix {
                control32::jb_rel32(machine, &full_bytes);
            } else {
                control::jb_rel16(machine, &full_bytes);
            }
        }
        0x83 => {
            if machine.has_operand_size_prefix {
                control32::jae_rel32(machine, &full_bytes);
            } else {
                control::jae_rel16(machine, &full_bytes);
            }
        }
        0x84 => {
            if machine.has_operand_size_prefix {
                control32::jz_rel32(machine, &full_bytes);
            } else {
                control::jz_rel16(machine, &full_bytes);
            }
        }
        0xB7 => {
            if machine.has_operand_size_prefix {
                extended32::movzx_r32_rm16(machine, &full_bytes);
            } else {
                extended::movzx_r16_rm8(machine, &full_bytes);
            }
        }
        _ => {
            log::error!(
                "Unsupported opcode0f {:#02X} at CS:IP = {:#04x}:{:#04x}",
                opcode,
                machine.registers.cs(),
                machine.registers.ip()
            );
            //machine.halted = true;
            crate::instructions::system::call_interrupt(machine, 0x06);
        }
    }
}
