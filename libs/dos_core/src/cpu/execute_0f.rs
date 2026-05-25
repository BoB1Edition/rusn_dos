use crate::{DosMachine, instructions::{control, control32, extended, extended32, stack}, modrm::ModRm};


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
            let modrm = ModRm::from_byte(modrm_byte);
            let disp_size = if modrm.mod_field != 0b11 {
                match modrm.mod_field {
                    0b00 => {
                        if modrm.rm_field == 0b110 {
                            2
                        } else {
                            0
                        }
                    }
                    0b01 => 1,
                    0b10 => 2,
                    _ => 0,
                }
            } else {
                0
            };

            match modrm.reg_field {
                0 => {
                    // SMSW r/m16/32 — smsw сама прочитает ModR/M и disp, продвинет IP
                    control::smsw(machine, &full_bytes);
                }
                1 => {
                    if modrm.mod_field == 0b00 && modrm.rm_field == 0b010 {
                        // LGDT stub — пропускаем ModR/M (+1) и disp (уже учтён)
                        machine.registers.step(Some(1 + disp_size));
                        log::warn!("LGDT stub: Ignored (Real Mode)");
                    } else {
                        // Другие /1 обрабатываем как SMSW (или оставляем smsw)
                        control::smsw(machine, &full_bytes);
                    }
                }
                2 => {
                    // LIDT/SIDT stub — пропускаем ModR/M и смещение
                    machine.registers.step(Some(1 + disp_size));
                    log::info!("LIDT/SIDT stub: Ignored (Real Mode)");
                }
                4 => {
                    // SMSW r/m16/32
                    control::smsw(machine, &full_bytes);
                }
                _ => {
                    // Нереализованные подфункции: пропускаем байты
                    machine.registers.step(Some(1 + disp_size));
                    log::warn!(
                        "Unhandled 0F 01 /{} at CS:IP={:#04x}:{:#04x}",
                        modrm.reg_field,
                        machine.registers.cs(),
                        machine.registers.ip()
                    );
                }
            }
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
            machine.halted = true;
        }
    }
}