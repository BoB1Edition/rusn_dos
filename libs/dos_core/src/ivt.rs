// Ver: 2 File: ./libs/dos_core/src/ivt.rs
use crate::DosMachine;

/// Базовый адрес IVT в физической памяти
const IVT_BASE: u32 = 0x00000;

/// Инициализирует таблицу векторов прерываний (IVT) в памяти по адресу 0x0000.
/// Заполняет стандартные обработчики DOS/BIOS.
/// 

pub(crate) fn init_ivt(machine: &mut DosMachine) {
    
    // Очищаем IVT (заполняем нулями)
    for i in 0..=255 {
        set_vector(machine, i, 0, 0);
    }

    set_vector(machine, 0x08, 0xF000, 0x0010); // IRQ0: PIT Timer
    set_vector(machine, 0x09, 0xF000, 0x0020); // IRQ1: Keyboard
    set_vector(machine, 0x10, 0xF000, 0x0030); // INT 10h: Video Services
    set_vector(machine, 0x1a, 0xF000, 0x0040); // INT 10h: Video Services
    set_vector(machine, 0x13, 0xF000, 0x0050); // INT 13h: Disk Services
    set_vector(machine, 0x15, 0xF000, 0x0060); // INT 15h: BIOS Extensions
    set_vector(machine, 0x16, 0xF000, 0x0070); // INT 16h: Keyboard Services
    set_vector(machine, 0x1C, 0xF000, 0x0080); // INT 1Ch: User Timer Tick
    set_vector(machine, 0x1F, 0xF000, 0x0090); // INT 1Fh: Video Graphics Table
    set_vector(machine, 0x20, 0xF000, 0x00A0); // INT 20h: Program Terminate
    set_vector(machine, 0x21, 0xF000, 0x00B0); // INT 21h: DOS API
    set_vector(machine, 0x2F, 0xF000, 0x00C0); // INT 2Fh: Multiplex
    set_vector(machine, 0x67, 0xF000, 0x00D0); // INT 67h: EMS (LIM 4.0)

    for vec in 0..256 {
        let handler_addr = 0xF0000 + (vec as u32 * 4); // Условное смещение для заглушек
        // В реальном эмуляторе здесь размещается x86-код трассировки/диспетчера
    }

    log::info!("IVT initialized at 0x00000. Default handlers set.");
}

fn set_vector(machine: &mut DosMachine, vector: u8, cs: u16, ip: u16) {
        let addr = IVT_BASE + (vector as u32) * 4;
        machine.write_phys_u16(addr, ip);
        machine.write_phys_u16(addr + 2, cs);
}