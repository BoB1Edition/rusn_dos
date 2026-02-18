// Ver: 2
use crate::{DosMachine, instructions::control, modrm::ModRm};

pub fn call_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);

    let target_addr = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u32(addr)
    };

    // В real mode: адрес усекается до 16 бит
    let target_ip = target_addr as u16;

    // PUSH текущего IP
    let current_ip = machine.registers.ip();
    machine.write_u16(
        machine.registers.ss(),
        machine.registers.sp().wrapping_sub(2),
        current_ip,
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));

    // JMP
    machine.registers.set_ip(target_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn jmp_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);

    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);

    let modrm = ModRm::from_byte(modrm_byte);
    let target_addr = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let addr = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        machine.read_phys_u32(addr)
    };
    let target_ip = target_addr as u16;
    machine.registers.set_ip(target_ip);

    machine.log_instruction(csip, &bytes).ok();
}

pub fn call32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    let rel32 = machine.read_u32(machine.registers.cs(), machine.registers.ip()) as i32;
    bytes.extend_from_slice(&rel32.to_le_bytes());
    machine.registers.step(Some(4));
    let offset16 = (rel32 & 0xFFFF) as i16;
    let return_ip = machine.registers.ip();
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), return_ip);
    let new_ip = (return_ip as i32 + offset16 as i32) as u16;
    machine.registers.set_ip(new_ip);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn retn32(machine: &mut DosMachine, prev: &[u8]) {
    control::retn(machine, prev);
}

/// JZ/JE rel32 — условный переход при ZF=1 с 32-битным смещением
/// В реальном режиме результат усекается до 16 бит (IP — 16-битный регистр)
pub fn jz_rel32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let mut bytes = prev.to_vec();
    
    // Читаем 32-битное смещение (sign-extended)
    let rel32 = machine.read_u32(machine.registers.cs(), machine.registers.ip()) as i32;
    machine.registers.step(Some(4));
    bytes.extend_from_slice(&rel32.to_le_bytes());
    
    // Проверяем флаг ZF (бит 6)
    let zf = (machine.registers.flags() & (1 << 6)) != 0;
    
    if zf {
        // Вычисляем новый IP: текущий IP (после чтения смещения) + sign_extend(rel32)
        // В реальном режиме усекаем до 16 бит
        let new_ip = (machine.registers.ip() as i32).wrapping_add(rel32) as u16;
        machine.registers.set_ip(new_ip);
    }
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn jecxz_rel8(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    // Читаем 8-битное смещение со знаком
    let rel8 = machine.read_u8(machine.registers.cs(), machine.registers.ip()) as i8;
    machine.registers.step(None); // продвигаем на 1 байт (смещение)
    let mut bytes = prev.to_vec();
    bytes.push(rel8 as u8);
    
    // Проверяем регистр ECX (НЕ флаги!)
    if machine.registers.ecx() == 0 {
        // Вычисляем новый EIP (в реальном режиме всё равно усекается до 16 бит)
        let new_eip = (machine.registers.ip() as i32).wrapping_add(rel8 as i32) as u32;
        machine.registers.set_ip((new_eip & 0xFFFF) as u16);
    }
    
    // Флаги НЕ изменяются
    machine.log_instruction(csip, &bytes).ok();
}

/// CALL ptr32:32 — Far call through memory (32-bit)
/// В реальном режиме не используется, но реализована для совместимости
pub fn call_far_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    let addr = if modrm.is_register_mode() {
        log::error!("CALL far through register is undefined behavior at {:#04x}:{:#04x}",
                    machine.registers.cs(), machine.registers.ip());
        machine.halted = true;
        return;
    } else {
        modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap()
    };
    // В реальном режиме нет EIP, только IP (16 бит)
    // Для 32-битной версии в реальном режиме используем 16-битное смещение
    let addr_u16 = addr as u16;
    let ip_offset = machine.read_u16(src_segment, addr_u16);
    let cs_segment = machine.read_u16(src_segment, addr_u16.wrapping_add(2));
    
    // Сохраняем текущий CS:IP в стек
    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_u16(machine.registers.ss(), sp, machine.registers.cs());
    
    let sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(sp);
    machine.write_u16(machine.registers.ss(), sp, machine.registers.ip());
    
    // Загружаем новый сегмент и смещение (в реальном режиме только 16 бит)
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    
    machine.log_instruction(csip, &bytes).ok();
}

pub fn jmp_far_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [machine.registers.cs(), machine.registers.ip()];
    let modrm_byte = machine.read_u8(machine.registers.cs(), machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    
    let src_segment = machine.override_segment.unwrap_or(machine.registers.ds());
    
    let addr = if modrm.is_register_mode() {
        log::error!("JMP far through register is undefined behavior at {:#04x}:{:#04x}",
            machine.registers.cs(), machine.registers.ip());
        machine.halted = true;
        return;
    } else {
        modrm.resolve_address(machine, machine.has_address_size_prefix, &mut bytes).unwrap()
    };
    
    let addr_u16 = addr as u16;
    let ip_offset = machine.read_u16(src_segment, addr_u16);  // ← 16 бит смещения
    let cs_segment = machine.read_u16(src_segment, addr_u16.wrapping_add(2));  // ← +2, НЕ +4!
    
    machine.registers.set_cs(cs_segment);
    machine.registers.set_ip(ip_offset);
    
    machine.log_instruction(csip, &bytes).ok();
}