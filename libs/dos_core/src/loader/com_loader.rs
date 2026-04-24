// Ver: 3
use std::fs::File;

use crate::{DosMachine, init_ivt, memory::Memory};

#[derive(Debug, Clone, Default)]
pub struct ComLoader {
    data: Vec<u8>,
}

impl ComLoader {
    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self { data })
    }

    pub fn exec(self, no_log: bool) -> Result<DosMachine, Box<dyn std::error::Error>> {
        const PSP_SEGMENT: u16 = 0x1000; // PSP всегда в сегменте 0
        const CODE_OFFSET: u16 = 0x0100; // Код начинается со смещения 0x100
        const STACK_TOP: u16 = 0xFFFE; // Вершина стека (64 КБ - 2)

        // 1. Создаём память и инициализируем нулями
        let mut memory = Memory::new();

        // 2. Создаём PSP (Program Segment Prefix) в сегменте 0x0000
        Self::create_psp(&mut memory, PSP_SEGMENT);

        // 3. Загружаем код по физическому адресу 0x0100 (сегмент 0x0000, смещение 0x0100)
        let code_base = (PSP_SEGMENT as u32 * 16 + CODE_OFFSET as u32) as usize;
        if code_base + self.data.len() > memory.len() {
            return Err("COM file too large (>64KB)".into());
        }
        for (i, &byte) in self.data.iter().enumerate() {
            memory.write_u8((code_base + i) as u32, byte);
        }

        memory.print_data(1);

        let logfile = if no_log {
            #[cfg(target_os = "windows")]
            {
                File::create("NUL")?
            }

            #[cfg(not(target_os = "windows"))]
            {
                File::create("/dev/null")?
            }
        } else {
            File::create("logopcode.txt")?
        };

        // 4. Инициализируем машину
        let mut machine = DosMachine::new_with_memory(memory, logfile);
        machine.memory.print_data(2);
        init_ivt(&mut machine);
        machine.memory.print_data(3);

        // 5. Устанавливаем регистры согласно спецификации DOS для .COM:
        //    - Все сегменты = 0 (включая PSP)
        //    - IP = 0x0100 (начало кода после PSP)
        //    - SP = 0xFFFE (стек растёт вниз от конца сегмента)
        machine.registers.set_cs(PSP_SEGMENT);
        machine.registers.set_ds(PSP_SEGMENT); // DS указывает на PSP (для доступа к параметрам)
        machine.registers.set_es(PSP_SEGMENT);
        machine.registers.set_ss(PSP_SEGMENT);
        machine.registers.set_ip(CODE_OFFSET);
        machine.registers.set_fs(PSP_SEGMENT);
        machine.registers.set_gs(PSP_SEGMENT);
        machine.registers.set_sp(STACK_TOP);

        log::info!(
            "Loaded .COM file: CS:IP={:04X}:{:04X}, SS:SP={:04X}:{:04X}, DS={:04X}",
            machine.registers.cs(),
            machine.registers.ip(),
            machine.registers.ss(),
            machine.registers.sp(),
            machine.registers.ds()
        );

        Ok(machine)
    }

    fn create_psp(memory: &mut Memory, segment: u16) {
        let psp_base = segment as u32 * 16;
        memory.write_u8(psp_base, 0xCD);
        memory.write_u8(psp_base + 1, 0x20);
        // Размер памяти в параграфах (640 КБ = 0xA000 параграфов)
        memory.write_u8(psp_base + 2, 0x00);
        memory.write_u8(psp_base + 3, 0xA0);
        // INT 21h / RETF — точка входа для вызовов DOS
        memory.write_u8(psp_base + 5, 0xCD);
        memory.write_u8(psp_base + 6, 0x21);
        memory.write_u8(psp_base + 7, 0xCB);
    }
}
