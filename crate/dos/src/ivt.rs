// Ver: 2 File: crate/dos/src/ivt.rs
use bus::Machine;

/// Базовый адрес IVT в физической памяти
const IVT_BASE: u32 = 0x00000;

/// Инициализирует таблицу векторов прерываний (IVT) в памяти по адресу 0x0000.
/// Заполняет стандартные обработчики DOS/BIOS.
pub fn init_ivt(machine: &mut dyn Machine) {
    // Очищаем IVT (заполняем нулями)
    for i in 0..=255 {
        set_vector(machine, i, 0, 0);
    }

    set_vector(machine, 0x08, 0xF000, 0x0010); // IRQ0: PIT Timer
    set_vector(machine, 0x09, 0xF000, 0x0020); // IRQ1: Keyboard
    set_vector(machine, 0x10, 0xF000, 0x0030); // INT 10h: Video Services
    set_vector(machine, 0x12, 0xF000, 0x0040);
    set_vector(machine, 0x1a, 0xF000, 0x0050);
    set_vector(machine, 0x13, 0xF000, 0x0060); // INT 13h: Disk Services
    set_vector(machine, 0x15, 0xF000, 0x0070); // INT 15h: BIOS Extensions
    set_vector(machine, 0x16, 0xF000, 0x0080); // INT 16h: Keyboard Services
    set_vector(machine, 0x1C, 0xF000, 0x0090); // INT 1Ch: User Timer Tick
    set_vector(machine, 0x1F, 0xF000, 0x00A0); // INT 1Fh: Video Graphics Table
    set_vector(machine, 0x20, 0xF000, 0x00B0); // INT 20h: Program Terminate
    set_vector(machine, 0x21, 0xF000, 0x00C0); // INT 21h: DOS API
    set_vector(machine, 0x2F, 0xF000, 0x00D0); // INT 2Fh: Multiplex
    set_vector(machine, 0x67, 0xF000, 0x0100); // INT 67h: EMS (LIM 4.0)

    log::info!("IVT initialized at 0x00000. Default handlers set.");
}

fn set_vector(machine: &mut dyn Machine, vector: u8, cs: u16, ip: u16) {
    let addr = IVT_BASE + (vector as u32) * 4;
    machine.write_mem_u16(addr, ip);
    machine.write_mem_u16(addr + 2, cs);
}

/// Проверяет, является ли вектор «внутренним» (обработчик DOS/BIOS, CS=0xF000).
pub fn is_internal(machine: &dyn Machine, vector: u8) -> bool {
    let addr = IVT_BASE + (vector as u32) * 4;
    machine.read_mem_u16(addr + 2) == 0xF000
}
