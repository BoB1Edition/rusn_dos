// Ver: 1
use crate::{DosMachine, loader::exe_header::MzHeader, memory::Memory};
use std::fs::File;

pub struct ExeLoader {
    header: MzHeader,
    data: Vec<u8>,
}

impl ExeLoader {
    pub fn from_file(path: &std::path::Path) -> crate::error::Result<Self> {
        let data = std::fs::read(path)?;
        let header = Self::parse_header(&data)?;
        Ok(Self { header, data })
    }

    fn parse_header(data: &[u8]) -> crate::error::Result<MzHeader> {
        if data.len() < 64 {
            return Err("File too short for MZ header".into());
        }
        if u16::from_le_bytes(data[0..2].try_into()?) != 0x5A4D {
            return Err("Not an MZ/EXE file".into());
        }
        // ... парсинг полей заголовка (аналогично текущему com.rs) ...
        Ok(MzHeader {
            e_sign: u16::from_le_bytes(data[0..2].try_into()?),
            e_cblp: u16::from_le_bytes(data[2..4].try_into()?),
            e_cp: u16::from_le_bytes(data[4..6].try_into()?),
            e_relc: u16::from_le_bytes(data[6..8].try_into()?),
            e_cparhdr: u16::from_le_bytes(data[8..10].try_into()?),
            e_minep: u16::from_le_bytes(data[10..12].try_into()?),
            e_maxep: u16::from_le_bytes(data[12..14].try_into()?),
            ss: u16::from_le_bytes(data[14..16].try_into()?),
            sp: u16::from_le_bytes(data[16..18].try_into()?),
            e_check: u16::from_le_bytes(data[18..20].try_into()?),
            ip: u16::from_le_bytes(data[20..22].try_into()?),
            cs: u16::from_le_bytes(data[22..24].try_into()?),
            e_lfarlc: u16::from_le_bytes(data[24..26].try_into()?),
            e_ovno: u16::from_le_bytes(data[26..28].try_into()?),
            e_res0x1c: vec![
                u16::from_le_bytes(data[28..30].try_into()?),
                u16::from_le_bytes(data[30..32].try_into()?),
                u16::from_le_bytes(data[32..34].try_into()?),
                u16::from_le_bytes(data[34..36].try_into()?),
            ]
            .as_slice()
            .try_into()?,
            e_oemid: u16::from_le_bytes(data[36..38].try_into()?),
            e_oeminfo: u16::from_le_bytes(data[38..40].try_into()?),
            e_res_0x28: vec![
                u16::from_le_bytes(data[40..42].try_into()?),
                u16::from_le_bytes(data[42..44].try_into()?),
                u16::from_le_bytes(data[44..46].try_into()?),
                u16::from_le_bytes(data[46..48].try_into()?),
                u16::from_le_bytes(data[48..50].try_into()?),
                u16::from_le_bytes(data[50..52].try_into()?),
                u16::from_le_bytes(data[52..54].try_into()?),
                u16::from_le_bytes(data[54..56].try_into()?),
                u16::from_le_bytes(data[56..58].try_into()?),
                u16::from_le_bytes(data[58..60].try_into()?),
            ]
            .as_slice()
            .try_into()?,
            e_lfanew: u32::from_le_bytes(data[60..64].try_into()?),
        })
    }

    pub fn exec(self, no_log: bool) -> Result<DosMachine, Box<dyn std::error::Error>> {
        const LOAD_SEGMENT: u16 = 0x1000; // Базовый сегмент загрузки
        const PSP_SEGMENT: u16 = LOAD_SEGMENT - 0x10; // PSP на 256 байт ниже кода

        // 1. Создаём память
        let mut memory = Memory::new();

        // 2. Создаём PSP
        Self::create_psp(&mut memory, PSP_SEGMENT);

        // 3. Загружаем код (пропускаем заголовок)
        let header_size = self.header.e_cparhdr as usize * 16;
        let code_data = &self.data[header_size..];
        let code_base = LOAD_SEGMENT as u32 * 16;
        if code_base as usize + code_data.len() > memory.len() {
            return Err("EXE file too large".into());
        }
        for (i, &byte) in code_data.iter().enumerate() {
            memory.write_u8(code_base + i as u32, byte);
        }

        // 4. Применяем релокации
        self.apply_relocations(&mut memory, LOAD_SEGMENT);

        let logfile = if no_log {
            File::create("/dev/null")?  // Unix
            // File::create("NUL")?     // Windows (раскомментировать при кроссплатформенности)
        } else {
            File::create("logopcode.txt")?
        };

        // 5. Инициализируем машину
        let mut machine = DosMachine::new_with_memory(memory, logfile);

        // 6. Устанавливаем регистры СОГЛАСНО СПЕЦИФИКАЦИИ DOS ДЛЯ .EXE:
        //    - CS:IP = заголовок (смещение от LOAD_SEGMENT)
        //    - SS:SP = заголовок (смещение от LOAD_SEGMENT)
        //    - DS = ES = LOAD_SEGMENT (сегмент ДАННЫХ программы, НЕ PSP!)
        //      Это критично: большинство программ ожидают DS указывать на их данные,
        //      а не на PSP. Доступ к PSP осуществляется через явное указание сегмента.
        let cs = LOAD_SEGMENT.wrapping_add(self.header.cs);
        let ip = self.header.ip;
        let ss = LOAD_SEGMENT.wrapping_add(self.header.ss);
        let sp = self.header.sp;

        machine.registers.set_cs(cs);
        machine.registers.set_ds(LOAD_SEGMENT); // ← КРИТИЧНО: не PSP, а сегмент данных!
        machine.registers.set_es(LOAD_SEGMENT); // ← КРИТИЧНО: не PSP, а сегмент данных!
        machine.registers.set_ss(ss);
        machine.registers.set_ip(ip);
        machine.registers.set_sp(sp);

        log::info!(
            "Loaded .EXE file: CS:IP={:04X}:{:04X}, SS:SP={:04X}:{:04X}, DS={:04X}",
            machine.registers.cs(),
            machine.registers.ip(),
            machine.registers.ss(),
            machine.registers.sp(),
            machine.registers.ds()
        );

        Ok(machine)
    }

    fn apply_relocations(&self, memory: &mut Memory, load_segment: u16) {
        let reloc_table_offset = self.header.e_lfarlc as usize; // Смещение в БАЙТАХ (не параграфах!)
        let reloc_count = self.header.e_relc as usize;

        for i in 0..reloc_count {
            let entry_offset = reloc_table_offset + i * 4;
            if entry_offset + 4 > self.data.len() {
                break;
            }

            // Читаем смещение и сегмент из таблицы релокаций
            let offset = u16::from_le_bytes([self.data[entry_offset], self.data[entry_offset + 1]]);
            let segment =
                u16::from_le_bytes([self.data[entry_offset + 2], self.data[entry_offset + 3]]);

            // Вычисляем физический адрес для релокации
            let fixup_addr = ((load_segment as u32 + segment as u32) << 4) + offset as u32;
            let current = memory.read_u16(fixup_addr);
            let corrected = current.wrapping_add(load_segment);

            // Записываем скорректированное значение
            memory.write_u16(fixup_addr, corrected);
        }
    }

    fn create_psp(memory: &mut Memory, segment: u16) {
        let psp_base = segment as u32 * 16;
        memory.write_u8(psp_base, 0xCD);
        memory.write_u8(psp_base + 1, 0x20);
        memory.write_u8(psp_base + 2, 0x00); // 640 КБ = 0xA000 параграфов
        memory.write_u8(psp_base + 3, 0xA0);
        memory.write_u8(psp_base + 8, 0xCD);
        memory.write_u8(psp_base + 9, 0x21);
        memory.write_u8(psp_base + 10, 0xCB);
    }
}
