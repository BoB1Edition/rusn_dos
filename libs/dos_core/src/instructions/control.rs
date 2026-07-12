// Ver: 2 File: ./libs/dos_core/src/instructions/control.rs
use crate::{flags, machine::DosMachine, modrm::ModRm};

pub(crate) fn call(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let rel16 = machine.read_instr_u16(machine.registers.ip()) as i16;
    bytes.extend_from_slice(&rel16.to_le_bytes());

    let return_ip = machine.registers.ip().wrapping_add(2);

    // x86: запись в стек только через SS физически
    let stack_addr = ((machine.registers.ss() as u32) << 4)
        .wrapping_add(machine.registers.sp().wrapping_sub(2) as u32);
    machine.write_phys_u16(stack_addr, return_ip);
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));

    let new_ip = (return_ip as i32 + rel16 as i32) as u16;
    machine.registers.set_ip(new_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn retn(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let stack_addr =
        ((machine.registers.ss() as u32) << 4).wrapping_add(machine.registers.sp() as u32);

    let ip = machine.read_phys_u16(stack_addr);
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    if csip[0] == 0x1000 && csip[1] == 0x900 {
        log::debug!("ip: {:04X}, stack_addr: {:04X}", ip, stack_addr)
    }
    machine.registers.set_ip(ip);

    machine.log_instruction(csip, prev).ok();
}

pub(crate) fn jz(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    bytes.push(rel8 as u8);
    let _ = machine.log_instruction(csip, &bytes);
    machine.registers.step(None);

    if (machine.registers.flags() & (flags::ZF)) != 0 {
        let current_ip = machine.registers.ip() as i32;
        let new_ip_32 = current_ip.wrapping_add(rel8 as i32);
        let new_ip = (new_ip_32 & 0xFFFF) as u16;
        machine.registers.set_ip(new_ip);
    }
}

pub(crate) fn ja(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    machine.log_instruction(csip, &bytes).ok();

    let flags = machine.registers.flags();
    let cf = (flags & 0x0001) != 0;
    let zf = (flags & 0x0040) != 0;

    if !cf && !zf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
}

pub(crate) fn call_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let target_ip = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .expect("Failed to resolve memory address in CALL r/m16");
        machine.read_phys_u16(addr)
    };

    if target_ip == 0 {
        log::error!(
            "CALL r/m16: target IP is zero at CS:IP={:04X}:{:04X}. Halting.",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.log_instruction(csip, &bytes).ok();
        machine.halted = true;
        return;
    }

    let current_ip = machine.registers.ip();
    let new_sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(new_sp);

    let stack_addr = ((machine.registers.ss() as u32) << 4).wrapping_add(new_sp as u32);
    machine.write_phys_u16(stack_addr, current_ip);

    machine.registers.set_ip(target_ip);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn smsw(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    if modrm.is_register_mode() {
        machine.write_reg16(modrm.rm_field, 0x0000);
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.write_phys_u16(addr, 0x0000);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jmp_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let target_ip = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u16(addr)
    };
    if target_ip == 0 {
        log::warn!(
            "JMP r/m16: target IP is zero at CS:IP={:04X}:{:04X}",
            machine.registers.cs(),
            machine.registers.ip()
        );
    }
    machine.registers.set_ip(target_ip);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jb(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    let cf = (machine.registers.flags() & 0x0001) != 0;
    machine.log_instruction(csip, &bytes).ok();

    if cf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
}

pub(crate) fn loop_cx(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    let cx = machine.registers.cx().wrapping_sub(1);
    machine.registers.set_cx(cx);
    if cx != 0 {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jmp_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
    machine.registers.set_ip(new_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jne_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаг нуля ZF (бит 6) — прыжок при ZF=0
    let zf = (machine.registers.flags() & (flags::ZF)) != 0;

    if !zf {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        // В реальном режиме усекаем до 16 бит
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}


pub(crate) fn jae_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаг переноса CF (бит 0) — прыжок при CF=0
    let cf = (machine.registers.flags() & 1) != 0;

    if !cf {
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}


pub(crate) fn jge_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    let flags = machine.registers.flags();
    let sf = (flags & (flags::SF)) != 0; // Sign Flag (бит 7)
    let of = (flags & (flags::OF)) != 0; // Overflow Flag (бит 11)

    if sf == of {
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}


pub(crate) fn jmp_rel16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel16 = machine.read_instr_u16(machine.registers.ip()) as i16;
    machine.registers.step(Some(2)); // продвигаем на 2 байта (смещение)
    let mut bytes = prev.to_vec();
    bytes.extend_from_slice(&rel16.to_le_bytes());

    // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel16)
    // В реальном режиме усекаем до 16 бит
    let new_ip = (machine.registers.ip() as i32).wrapping_add(rel16 as i32) as u16;
    machine.registers.set_ip(new_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jcxz_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем регистр CX (НЕ флаги!)
    if machine.registers.cx() == 0 {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        // В реальном режиме усекаем до 16 бит
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn loopnz_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    let cx = machine.registers.cx();
    let new_cx = cx.wrapping_sub(1);
    machine.registers.set_cx(new_cx);

    let zf = (machine.registers.flags() & (flags::ZF)) != 0;
    if new_cx != 0 && !zf {
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn loopz_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // 1. Декрементируем CX (флаги НЕ устанавливаются!)
    let cx = machine.registers.cx();
    let new_cx = cx.wrapping_sub(1);
    machine.registers.set_cx(new_cx);

    // 2. Проверяем флаг нуля ZF (бит 6)
    let zf = (machine.registers.flags() & (flags::ZF)) != 0;

    // 3. Выполняем прыжок если CX ≠ 0 И ZF = 1 (равно)
    if new_cx != 0 && zf {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    // Флаги НЕ изменяются — критически важно! (декремент не устанавливает флаги)
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jl_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаги знака (SF, бит 7) и переполнения (OF, бит 11)
    let flags = machine.registers.flags();
    let sf = (flags & (flags::SF)) != 0;
    let of = (flags & (flags::OF)) != 0;

    // Условие перехода: SF ≠ OF (знаковый результат отрицательный → "меньше")
    if sf != of {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel8)
        let new_ip = (machine.registers.ip() as i16).wrapping_add(rel8 as i16) as u16;
        machine.registers.set_ip(new_ip);
    }

    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn call_far(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    bytes.push(0x9A); // опкод CALL far

    let ip_offset = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&ip_offset.to_le_bytes());

    let cs_segment = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&cs_segment.to_le_bytes());

    let sp = machine.registers.sp().wrapping_sub(2);
    let stack_addr_cs = ((machine.registers.ss() as u32) << 4).wrapping_add(sp as u32);
    machine.write_phys_u16(stack_addr_cs, machine.registers.cs());
    machine.registers.set_sp(sp);

    // 2. Сохраняем IP
    let sp = machine.registers.sp().wrapping_sub(2);
    let stack_addr_ip = ((machine.registers.ss() as u32) << 4).wrapping_add(sp as u32);
    machine.write_phys_u16(stack_addr_ip, machine.registers.ip());
    machine.registers.set_sp(sp);

    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jns_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаг знака SF (бит 7)
    let sf = (machine.registers.flags() & (flags::SF)) != 0;

    // Условие перехода: SF = 0 (результат неотрицательный)
    if !sf {
        let current_ip = machine.registers.ip() as i32;
        let new_ip_32 = current_ip.wrapping_add(rel8 as i32);
        let new_ip = (new_ip_32 & 0xFFFF) as u16;

        machine.registers.set_ip(new_ip);
    }

    // Флаги НЕ изменяются — критически важно!
    machine.log_instruction(csip, &bytes).ok();
}


pub(crate) fn jg_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    let flags = machine.registers.flags();
    let zf = (flags & (flags::ZF)) != 0;
    let sf = (flags & (flags::SF)) != 0;
    let of = (flags & (flags::OF)) != 0;
    if !zf && (sf == of) {
        let current_ip = machine.registers.ip() as i32;
        let new_ip_32 = current_ip.wrapping_add(rel8 as i32);
        let new_ip = (new_ip_32 & 0xFFFF) as u16;

        machine.registers.set_ip(new_ip);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jmp_far(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();

    let ip_offset = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&ip_offset.to_le_bytes());

    let cs_segment = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&cs_segment.to_le_bytes());

    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn retf(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();

    let stack_addr_ip =
        ((machine.registers.ss() as u32) << 4).wrapping_add(machine.registers.sp() as u32);
    let ip = machine.read_phys_u16(stack_addr_ip);
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));

    // 2. Извлекаем CS
    let stack_addr_cs =
        ((machine.registers.ss() as u32) << 4).wrapping_add(machine.registers.sp() as u32);
    let cs = machine.read_phys_u16(stack_addr_cs);
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));

    machine.registers.set_ip(ip);
    machine.registers.set_cs(cs);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jle_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    let flags = machine.registers.flags();
    let zf = (flags & (flags::ZF)) != 0;
    let sf = (flags & (flags::SF)) != 0;
    let of = (flags & (flags::OF)) != 0;

    if zf || (sf != of) {
        let current_ip = machine.registers.ip() as i32;
        let new_ip_32 = current_ip.wrapping_add(rel8 as i32);
        let new_ip = (new_ip_32 & 0xFFFF) as u16;

        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn call_far_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let addr = if modrm.is_register_mode() {
        log::error!(
            "CALL far through register is undefined behavior at {:#04x}:{:#04x}",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.halted = true;
        return;
    } else {
        modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap()
    };
    let ip_offset = machine.read_phys_u16(addr);
    let cs_segment = machine.read_phys_u16(addr + 2);

    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    let ss_base = (machine.registers.ss() as u32) << 4;
    machine.write_phys_u16(ss_base.wrapping_add(sp as u32), machine.registers.cs());

    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_phys_u16(ss_base.wrapping_add(sp as u32), machine.registers.ip());

    // 3. Загружаем новый сегмент и смещение
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    // Флаги НЕ изменяются
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jmp_far_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let addr_offset = if modrm.is_register_mode() {
        log::error!(
            "JMP far through register is undefined behavior at {:#04x}:{:#04x}",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.halted = true;
        return;
    } else {
        modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap()
    };
    let ip_offset = machine.read_phys_u16(addr_offset);
    let cs_segment = machine.read_phys_u16(addr_offset.wrapping_add(2));
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jz_rel16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let rel16 = machine.read_instr_u16(machine.registers.ip()) as i16;
    machine.registers.step(Some(2)); // ← 2 байта для rel16!

    bytes.extend_from_slice(&rel16.to_le_bytes());

    // Проверяем флаг ZF (бит 6)
    let zf = (machine.registers.flags() & (flags::ZF)) != 0;

    if zf {
        // Вычисляем новый IP: текущий IP + sign_extend(rel16)
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel16 as i32) as u16;
        machine.registers.set_ip(new_ip);
        log::debug!("JZ rel16: TAKEN (ZF=1), new IP={:#04x}", new_ip);
    } else {
        log::debug!("JZ rel16: NOT TAKEN (ZF=0), continue");
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jae_rel16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let rel16 = machine.read_instr_u16(machine.registers.ip()) as i16;
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&rel16.to_le_bytes());

    let cf = (machine.registers.flags() & flags::CF) == 0;

    if cf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel16 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jb_rel16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();

    let rel16 = machine.read_instr_u16(machine.registers.ip()) as i16;
    machine.registers.step(Some(2));
    bytes.extend_from_slice(&rel16.to_le_bytes());

    let cf = (machine.registers.flags() & flags::CF) != 0;
    if cf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel16 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub fn bound_r16_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    if modrm.is_register_mode() {
        log::error!(
            "BOUND with register operand is undefined at CS:IP={:#04x}:{:#04x}",
            machine.registers.cs(),
            machine.registers.ip()
        );
        machine.halted = true;
        return;
    }
    let addr = modrm
        .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
        .unwrap();
    let low = machine.read_phys_u16(addr) as i16;
    let high = machine.read_phys_u16(addr.wrapping_add(2)) as i16;
    let reg_val = machine.read_reg16(modrm.reg_field) as i16;
    if reg_val < low || reg_val > high {
        log::warn!(
            "BOUND range exceeded: reg={} not in [{}, {}] at CS:IP={:#04x}:{:#04x}",
            reg_val,
            low,
            high,
            csip[0],
            csip[1]
        );
        crate::instructions::system::int(machine, &[0xCD, 0x05]);
        return;
    }

    machine.log_instruction(csip, &bytes).ok();
}

/// JO rel8 — Jump if Overflow (OF=1)
pub(crate) fn jo_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    let of = (machine.registers.flags() & (flags::OF)) != 0;
    if of {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn jbe_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    let flags = machine.registers.flags();
    let cf = (flags & (flags::CF)) != 0;
    let zf = (flags & (flags::ZF)) != 0;

    if cf || zf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn retn_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let mut bytes = prev.to_vec();
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    bytes.extend_from_slice(&imm16.to_le_bytes());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let stack_addr =
        ((machine.registers.ss() as u32) << 4).wrapping_add(machine.registers.sp() as u32);
    let ip = machine.read_phys_u16(stack_addr);
    let new_sp = machine.registers.sp().wrapping_add(2 + imm16);
    machine.registers.set_sp(new_sp);

    machine.registers.set_ip(ip);
    machine.log_instruction(csip, &bytes).ok();
}

/// JNO rel8 — Jump if Not Overflow (OF=0)
pub(crate) fn jno_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    let of = (machine.registers.flags() & (crate::flags::OF)) != 0;
    if !of {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
    }
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn leave(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let bytes = prev.to_vec();
    
    let ss_base = (machine.registers.ss() as u32) << 4;
    
    if machine.has_operand_size_prefix {
        // === 32-битная версия: MOV ESP, EBP; POP EBP ===
        let ebp = machine.registers.ebp();
        machine.registers.set_esp(ebp);
        
        // Читаем старое значение EBP из стека
        let esp = machine.registers.esp();
        let old_ebp = machine.read_phys_u32(ss_base.wrapping_add(esp as u32));
        machine.registers.set_ebp(old_ebp);
        machine.registers.set_esp(esp.wrapping_add(4));
        
        log::trace!(
            "LEAVE (32-bit): EBP={:#010x}, ESP={:#010x} -> {:#010x}",
            old_ebp, ebp, machine.registers.esp()
        );
    } else {
        // === 16-битная версия: MOV SP, BP; POP BP ===
        let bp = machine.registers.bp();
        machine.registers.set_sp(bp);
        
        // Читаем старое значение BP из стека
        let sp = machine.registers.sp();
        let old_bp = machine.read_phys_u16(ss_base.wrapping_add(sp as u32));
        machine.registers.set_bp(old_bp);
        machine.registers.set_sp(sp.wrapping_add(2));
        
        log::trace!(
            "LEAVE (16-bit): BP={:#06x}, SP={:#06x} -> {:#06x}",
            old_bp, bp, machine.registers.sp()
        );
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn js_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let rel8 = machine.read_instr_u8(machine.registers.ip()) as i8;
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);

    // Проверяем флаг знака SF (бит 7)
    let sf = (machine.registers.flags() & flags::SF) != 0;
    if sf {
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u16;
        machine.registers.set_ip(new_ip);
        log::debug!("JS rel8: TAKEN (SF=1), new IP={:#04x}", new_ip);
    } else {
        log::debug!("JS rel8: NOT TAKEN (SF=0), continue");
    }

    machine.log_instruction(csip, &bytes).ok();
}