// Ver: 3
use crate::machine::DosMachine;

pub fn push_cs(machine: &mut DosMachine) {
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), machine.registers.cs());
}

pub fn push_ax(machine: &mut DosMachine) {
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), machine.registers.ax());
}

pub fn push_bx(machine: &mut DosMachine) {
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), machine.registers.bx());
}

pub fn pushf(machine: &mut DosMachine) {
    machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
    machine.write_u16(machine.registers.ss(), machine.registers.sp(), machine.registers.flags());
}

pub fn pop_ds(machine: &mut DosMachine) { 
    let ds = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ds(ds);
}

pub fn pop_ax(machine: &mut DosMachine) {
    let ax = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ax(ax);
}

pub fn popf(machine: &mut DosMachine) {
    let flags = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_flags(flags);
}

pub fn pop_fs(machine: &mut DosMachine) {
    let fs = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_fs(fs);
}

// libs/dos_core/src/instructions/stack.rs
pub fn pusha(machine: &mut DosMachine) {
    // Сохраняем оригинальное значение SP ДО начала операции
    let original_sp = machine.registers.sp();
    
    // Порядок сохранения: AX, CX, DX, BX, original_SP, BP, SI, DI
    let regs = [
        machine.registers.ax(),
        machine.registers.cx(),
        machine.registers.dx(),
        machine.registers.bx(),
        original_sp,  // ← ВАЖНО: оригинальное значение SP!
        machine.registers.bp(),
        machine.registers.si(),
        machine.registers.di(),
    ];
    
    // Сохраняем регистры в обратном порядке (последний регистр — самый глубокий в стеке)
    for &reg in regs.iter().rev() {
        machine.registers.set_sp(machine.registers.sp().wrapping_sub(2));
        machine.write_u16(machine.registers.ss(), machine.registers.sp(), reg);
    }
    
    // Логирование
    let csip = [machine.registers.cs(), machine.registers.ip()];
    machine.log_instruction(csip, &[0x60]).ok();
}

pub fn pushad(machine: &mut DosMachine) {
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
    for &reg in regs.iter().rev() {
        machine.registers.set_esp(machine.registers.esp().wrapping_sub(4));
        let sp = machine.registers.sp();
        machine.write_phys_u32((machine.registers.ss() as u32 * 16 + sp as u32) & 0xFFFFF, reg);
    }
    let csip = [machine.registers.cs(), machine.registers.ip()];
    machine.log_instruction(csip, &[0x66, 0x60]).ok();
}

pub fn popa(machine: &mut DosMachine) {
    // Восстанавливаем в обратном порядке сохранения
    let di = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_di(di);
    
    let si = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_si(si);
    
    let bp = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_bp(bp);
    
    // Пропускаем оригинальное значение SP (сохранённое при PUSHA)
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    
    let bx = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_bx(bx);
    
    let dx = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_dx(dx);
    
    let cx = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_cx(cx);
    
    let ax = machine.read_u16(machine.registers.ss(), machine.registers.sp());
    machine.registers.set_sp(machine.registers.sp().wrapping_add(2));
    machine.registers.set_ax(ax);
    
    // Логирование
    let csip = [machine.registers.cs(), machine.registers.ip()];
    machine.log_instruction(csip, &[0x61]).ok();
}

/// POPAD — восстановление всех 32-битных регистров из стека
/// Аналогично POPA, но с 32-битными значениями и игнорированием оригинального ESP
pub fn popad(machine: &mut DosMachine) {
    let edi = machine.read_u32(
        machine.registers.ss(),
        machine.registers.sp()
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_add(4));
    machine.registers.set_edi(edi);
    
    let esi = machine.read_u32(
        machine.registers.ss(),
        machine.registers.sp()
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_add(4));
    machine.registers.set_esi(esi);
    
    let ebp = machine.read_u32(
        machine.registers.ss(),
        machine.registers.sp()
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_add(4));
    machine.registers.set_ebp(ebp);
    
    // Пропускаем оригинальное значение ESP (сохранённое при PUSHAD)
    machine.registers.set_sp(machine.registers.sp().wrapping_add(4));
    
    let ebx = machine.read_u32(
        machine.registers.ss(),
        machine.registers.sp()
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_add(4));
    machine.registers.set_ebx(ebx);
    
    let edx = machine.read_u32(
        machine.registers.ss(),
        machine.registers.sp()
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_add(4));
    machine.registers.set_edx(edx);
    
    let ecx = machine.read_u32(
        machine.registers.ss(),
        machine.registers.sp()
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_add(4));
    machine.registers.set_ecx(ecx);
    
    let eax = machine.read_u32(
        machine.registers.ss(),
        machine.registers.sp()
    );
    machine.registers.set_sp(machine.registers.sp().wrapping_add(4));
    machine.registers.set_eax(eax);
    
    // Логирование
    let csip = [machine.registers.cs(), machine.registers.ip()];
    machine.log_instruction(csip, &[0x66, 0x61]).ok();
}