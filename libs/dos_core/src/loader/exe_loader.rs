// Ver: 1 File: ./libs/dos_core/src/loader/exe_loader.rs
use crate::{DosMachine, init_ivt, loader::exe_header::MzHeader, memory::Memory};
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

    pub fn exec(&mut self, no_log: bool) -> Result<DosMachine, Box<dyn std::error::Error>> {
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

        let loaded_len = code_data.len();
let bss_paragraphs = self.header.e_minep as usize; // или e_maxep
if bss_paragraphs > 0 {
    let bss_start = code_base + loaded_len as u32;
    let bss_size = bss_paragraphs * 16;
    for i in 0..bss_size {
        if (bss_start as usize + i) < memory.len() {
            memory.write_u8(bss_start + i as u32, 0);
        }
    }
}
log::info!(
    "EXE load: file_size={}, header_size={}, code_data_len={}, expected_offset=0x274C",
    self.data.len(),
    header_size,
    code_data.len(),
);
if code_data.len() <= 0x274C {
    log::warn!("File does not contain data at header-relative offset 0x274C!");
}
let check_addr = 0x1274C;
let expected_offset_in_file = 0x27AC;
let file_byte_0 = self.data[expected_offset_in_file as usize];
let file_byte_1 = self.data[(expected_offset_in_file + 1) as usize];
let mem_byte_0 = memory.read_u8(check_addr);
let mem_byte_1 = memory.read_u8(check_addr + 1);

log::debug!(
    "COPY VERIFY: file[{:#04X}]={:02X}{:02X} → mem[{:#05X}]={:02X}{:02X}",
    expected_offset_in_file,
    file_byte_1, file_byte_0,  // little-endian display
    check_addr,
    mem_byte_1, mem_byte_0
);
if file_byte_0 != mem_byte_0 || file_byte_1 != mem_byte_1 {
    log::error!("COPY MISMATCH at {:#05X}!", check_addr);
}
        // 4. Применяем релокации
        self.apply_relocations(&mut memory, LOAD_SEGMENT);

        let logfile = if no_log {
            File::create("/dev/null")? // Unix
        // File::create("NUL")?     // Windows (раскомментировать при кроссплатформенности)
        } else {
            File::create("logopcode.txt")?
        };

        // 5. Инициализируем машину
        let mut machine = DosMachine::new_with_memory(memory, logfile);
        init_ivt(&mut machine);
        let cs = LOAD_SEGMENT.wrapping_add(self.header.cs);
        let ip = self.header.ip;
        let ss = LOAD_SEGMENT.wrapping_add(self.header.ss);
        let sp = self.header.sp;

        machine.registers.set_cs(cs);
        machine.registers.set_ds(PSP_SEGMENT);
        machine.registers.set_es(PSP_SEGMENT);
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
        let reloc_table_offset = self.header.e_lfarlc as usize;
        let reloc_count = self.header.e_relc as usize;

        log::debug!(
            "Applying {} relocations at offset {:#x}",
            reloc_count,
            reloc_table_offset
        );

        for i in 0..reloc_count {
            let entry_offset = reloc_table_offset + i * 4;

            if entry_offset + 4 > self.data.len() {
                log::warn!("Relocation table truncated at entry {}", i);
                break;
            }

            let offset = u16::from_le_bytes([self.data[entry_offset], self.data[entry_offset + 1]]);
            let segment =
                u16::from_le_bytes([self.data[entry_offset + 2], self.data[entry_offset + 3]]);

            let fixup_addr = ((load_segment as u32 + segment as u32) << 4) + offset as u32;

            // Проверка: не выходит ли адрес за пределы эмулируемой памяти
            if fixup_addr as usize + 2 > memory.len() {
                log::warn!(
                    "Relocation #{i}: address {:#06x} out of bounds (memory: {} bytes), skipping",
                    fixup_addr,
                    memory.len()
                );
                continue;
            }

            let current = memory.read_u16(fixup_addr);
            let corrected = current.wrapping_add(load_segment);
            memory.write_u16(fixup_addr, corrected);

            log::trace!(
                "Reloc #{i}: {:#04x}:{:#04x} → {:#06x}: {:#04x}→{:#04x}",
                segment,
                offset,
                fixup_addr,
                current,
                corrected
            );
        }
    }

    fn create_psp(memory: &mut Memory, segment: u16) {
        let psp_base = segment as u32 * 16;
        memory.write_u8(psp_base, 0xCD);
        memory.write_u8(psp_base + 1, 0x20);
        memory.write_u8(psp_base + 2, 0x00); // 640 КБ = 0xA000 параграфов
        memory.write_u8(psp_base + 3, 0xA0);
        memory.write_u8(psp_base + 5, 0xCD);
        memory.write_u8(psp_base + 6, 0x21);
        memory.write_u8(psp_base + 7, 0xCB);
    }
}
