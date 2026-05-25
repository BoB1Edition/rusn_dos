// Ver: 1 File: ./libs/dos_core/src/instructions/stack.rs
use crate::{machine::DosMachine, modrm::ModRm, pop_reg16, push_reg16};

pub(crate) fn push_cs(machine: &mut DosMachine) {
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(
        machine.registers.ss(),
        machine.registers.sp(),
        machine.registers.cs(),
    );
}

/*pub(crate) fn push_ax(machine: &mut DosMachine) {
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(
        machine.registers.ss(),
        machine.registers.sp(),
        machine.registers.ax(),
    );
}*/

pub(crate) fn push_es(machine: &mut DosMachine) {
    let es = machine.registers.es();
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), es);
}

pub(crate) fn pushf(machine: &mut DosMachine) {
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(
        machine.registers.ss(),
        machine.registers.sp(),
        machine.registers.flags(),
    );
}

/*pub(crate) fn pop_ds(machine: &mut DosMachine) {
    let ds = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ds(ds);
}
*/
pub(crate) fn popf(machine: &mut DosMachine) {
    let flags = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_flags(flags);
}

pub(crate) fn pop_fs(machine: &mut DosMachine) {
    let fs = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_fs(fs);
}

// libs/dos_core/src/instructions/stack.rs
pub(crate) fn pusha(machine: &mut DosMachine) {
    // Сохраняем оригинальное значение SP ДО начала операции
    let original_sp = machine.registers.sp();

    // Порядок сохранения: AX, CX, DX, BX, original_SP, BP, SI, DI
    let regs = [
        machine.registers.ax(),
        machine.registers.cx(),
        machine.registers.dx(),
        machine.registers.bx(),
        original_sp, // ← ВАЖНО: оригинальное значение SP!
        machine.registers.bp(),
        machine.registers.si(),
        machine.registers.di(),
    ];

    // Сохраняем регистры в обратном порядке (последний регистр — самый глубокий в стеке)
    for &reg in regs.iter().rev() {
        machine
            .registers
            .set_sp(machine.registers.sp().wrapping_sub(2));
        machine.write_u16(machine.registers.ss(), machine.registers.sp(), reg);
    }

    // Логирование
    let csip = [machine.registers.cs(), machine.registers.ip() - 1];
    machine.log_instruction(csip, &[0x60]).ok();
}

pub(crate) fn pushad(machine: &mut DosMachine) {
    let original_esp = machine.registers.esp();
    let regs = [
        machine.registers.eax(),
        machine.registers.ecx(),
        machine.registers.edx(),
        machine.registers.ebx(),
        original_esp,
        machine.registers.ebp(),
        machine.registers.esi(),
        machine.registers.edi(),
    ];
    let base = (machine.registers.ss() as u32) << 4;
    for &reg in regs.iter().rev() {
        let new_esp = machine.registers.esp().wrapping_sub(4);
        machine.registers.set_esp(new_esp);
        let phys_addr = base.wrapping_add(new_esp); // A20 применится в write_phys_u32
        machine.write_phys_u32(phys_addr, reg);
    }
    let csip = [machine.registers.cs(), machine.registers.ip() - 1];
    machine.log_instruction(csip, &[0x66, 0x60]).ok();
}

pub(crate) fn popa(machine: &mut DosMachine) {
    // Восстанавливаем в обратном порядке сохранения
    let di = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_di(di);

    let si = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_si(si);

    let bp = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_bp(bp);

    // Пропускаем оригинальное значение SP (сохранённое при PUSHA)
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));

    let bx = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_bx(bx);

    let dx = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_dx(dx);

    let cx = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_cx(cx);

    let ax = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine
        .registers
        .set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ax(ax);

    // Логирование
    let csip = [machine.registers.cs(), machine.registers.ip() - 1];
    machine.log_instruction(csip, &[0x61]).ok();
}

pub(crate) fn popad(machine: &mut DosMachine) {
    let base = (machine.registers.ss() as u32) << 4;
    let mut esp = machine.registers.esp();

    // Читаем в порядке: EDI, ESI, EBP, [skip ESP], EBX, EDX, ECX, EAX
    let edi = machine.read_phys_u32(base.wrapping_add(esp));
    esp = esp.wrapping_add(4);
    let esi = machine.read_phys_u32(base.wrapping_add(esp));
    esp = esp.wrapping_add(4);
    let ebp = machine.read_phys_u32(base.wrapping_add(esp));
    esp = esp.wrapping_add(4);

    // Пропускаем сохранённое значение ESP (стандартное поведение POPAD)
    esp = esp.wrapping_add(4);

    let ebx = machine.read_phys_u32(base.wrapping_add(esp));
    esp = esp.wrapping_add(4);
    let edx = machine.read_phys_u32(base.wrapping_add(esp));
    esp = esp.wrapping_add(4);
    let ecx = machine.read_phys_u32(base.wrapping_add(esp));
    esp = esp.wrapping_add(4);
    let eax = machine.read_phys_u32(base.wrapping_add(esp));
    esp = esp.wrapping_add(4);

    // Восстанавливаем регистры
    machine.registers.set_edi(edi);
    machine.registers.set_esi(esi);
    machine.registers.set_ebp(ebp);
    machine.registers.set_esp(esp);
    machine.registers.set_ebx(ebx);
    machine.registers.set_edx(edx);
    machine.registers.set_ecx(ecx);
    machine.registers.set_eax(eax);

    let csip = [machine.registers.cs(), machine.registers.ip() - 1];
    machine.log_instruction(csip, &[0x66, 0x61]).ok();
}

/*pub(crate) fn push_ds(machine: &mut DosMachine) {
    let new_sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(new_sp);
    let ds = machine.registers.ds();
    machine.write_u16(machine.registers.ss(), new_sp, ds);
}*/

pub(crate) fn pop_es(machine: &mut DosMachine) {
    let csip = [machine.registers.cs(), machine.registers.ip() - 1];
    let es_value = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    let new_sp = machine.registers.sp().wrapping_add(2);
    machine.registers.set_sp(new_sp);
    machine.registers.set_es(es_value);
    machine.log_instruction(csip, &[0x07]).ok();
}

// libs/dos_core/src/instructions/stack.rs
pub(crate) fn push_imm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];

    // Читаем 16-битное непосредственное значение (little-endian)
    let imm16 = machine.read_instr_u16(machine.registers.ip());
    machine.registers.step(Some(2)); // продвигаем на 2 байта (imm16)
    let mut bytes = prev.to_vec();
    bytes.push(0x68);
    bytes.extend_from_slice(&imm16.to_le_bytes());

    // Уменьшаем SP на 2 (стек растёт вниз)
    let new_sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(new_sp);

    // Записываем значение в стек по адресу [SS:SP]
    machine.write_u16(machine.registers.ss(), new_sp, imm16);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn push_imm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let imm32 = machine.read_instr_u32(machine.registers.ip());
    machine.registers.step(Some(4));

    let new_esp = machine.registers.esp().wrapping_sub(4);
    machine.registers.set_esp(new_esp);
    let phys_addr = ((machine.registers.ss() as u32) << 4).wrapping_add(new_esp);
    machine.write_phys_u32(phys_addr, imm32);

    machine.log_instruction(csip, prev).ok();
}

/// POP r/m16 — Pop word from stack into register or memory
/// Извлекает слово из стека и сохраняет его напрямую в регистр или память
pub(crate) fn pop_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    // Проверяем код операции (должен быть /0 для POP)
    if modrm.reg_field != 0 {
        log::error!(
            "Invalid opcode extension for 0x8F at {:#04x}:{:#04x} (expected /0, got /{})",
            machine.registers.cs(),
            machine.registers.ip(),
            modrm.reg_field
        );
        machine.halted = true;
        return;
    }

    let sp = machine.registers.sp();
    let value = machine.read_u16(machine.registers.ss(), sp);

    if modrm.is_register_mode() {
        machine.write_reg16(modrm.rm_field, value);
    } else {
        let offset = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        //let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        //let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.write_phys_u16(offset, value);
    }

    let new_sp = sp.wrapping_add(2);
    machine.registers.set_sp(new_sp);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn push_rm16(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);

    let new_sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(new_sp);

    let value = if modrm.is_register_mode() {
        machine.read_reg16(modrm.rm_field)
    } else {
        let offset = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        //let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        //let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u16(offset)
    };

    machine.write_u16(machine.registers.ss(), new_sp, value);

    machine.log_instruction(csip, &bytes).ok();
}

pub(crate) fn push_rm32(machine: &mut DosMachine, prev: &[u8]) {
    let csip = [
        machine.registers.cs(),
        machine.registers.ip() - prev.len() as u16,
    ];
    let modrm_byte = machine.read_instr_u8(machine.registers.ip());
    machine.registers.step(None);
    let mut bytes = prev.to_vec();
    bytes.push(modrm_byte);
    let modrm = ModRm::from_byte(modrm_byte);
    let new_esp = machine.registers.esp().wrapping_sub(4);
    machine.registers.set_esp(new_esp);
    let value = if modrm.is_register_mode() {
        machine.read_reg32(modrm.rm_field)
    } else {
        let offset = modrm
            .resolve_address(machine, machine.has_address_size_prefix, &mut bytes)
            .unwrap();
        //let segment = machine.override_segment.unwrap_or(machine.registers.ds());
        //let phys_addr = ((segment as u32) << 4).wrapping_add(offset as u32);
        machine.read_phys_u32(offset)
    };
    let phys_addr = ((machine.registers.ss() as u32) << 4).wrapping_add(new_esp);
    machine.write_phys_u32(phys_addr, value);

    machine.log_instruction(csip, &bytes).ok();
}

push_reg16!(push_ax, ax);
push_reg16!(push_cx, cx);
push_reg16!(push_dx, dx);
push_reg16!(push_bx, bx);
push_reg16!(push_si, si);
push_reg16!(push_di, di);
push_reg16!(push_sp, sp);
push_reg16!(push_bp, bp);

pop_reg16!(pop_ax, set_ax);
pop_reg16!(pop_cx, set_cx);
pop_reg16!(pop_dx, set_dx);
pop_reg16!(pop_bx, set_bx);
pop_reg16!(pop_si, set_si);
pop_reg16!(pop_di, set_di);
pop_reg16!(pop_sp, set_sp);
pop_reg16!(pop_bp, set_bp);

pub(crate) fn push_ds(machine: &mut DosMachine) {
    let csip = [machine.registers.cs(), machine.registers.ip() - 1];
    let new_sp = machine.registers.sp().wrapping_sub(2);
    machine.registers.set_sp(new_sp);
    let ds = machine.registers.ds();
    machine.write_u16(machine.registers.ss(), new_sp, ds);
    machine.log_instruction(csip, &[0x1E]).ok();
}

pub(crate) fn pop_ds(machine: &mut DosMachine) {
    let csip = [machine.registers.cs(), machine.registers.ip() - 1];
    let sp = machine.registers.sp();
    let ds = machine.read_u16(machine.registers.ss(), sp);
    machine.registers.set_sp(sp.wrapping_add(2));
    machine.registers.set_ds(ds);
    machine.log_instruction(csip, &[0x1F]).ok();
}