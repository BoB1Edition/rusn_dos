// Ver: 4 File: crate/dos/src/loader/exe_loader.rs
use crate::consts::{DOS_MEMORY_SIZE, MAX_MCB_SEGMENT, MCB_SIGNATURE_LAST, MCB_SIGNATURE_NON_LAST};
use crate::ivt::init_ivt;
use crate::loader::exe_header::MzHeader;
use crate::mcb::MCB;
use crate::system::DosSystem;
use bus::{Machine, Motherboard};

use emu_config::MachineConfig;
// ✅ ДОБАВЛЕНО: Правильный импорт из крейта x86
use x86::instructions::stack;

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
            e_res0x1c: [
                u16::from_le_bytes(data[28..30].try_into()?),
                u16::from_le_bytes(data[30..32].try_into()?),
                u16::from_le_bytes(data[32..34].try_into()?),
                u16::from_le_bytes(data[34..36].try_into()?),
            ],
            e_oemid: u16::from_le_bytes(data[36..38].try_into()?),
            e_oeminfo: u16::from_le_bytes(data[38..40].try_into()?),
            e_res_0x28: [
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
            ],
            e_lfanew: u32::from_le_bytes(data[60..64].try_into()?),
        })
    }

    pub fn exec(
        &mut self,
        no_log: bool,
        config: &MachineConfig,
    ) -> Result<DosSystem, Box<dyn std::error::Error>> {
        const LOAD_SEGMENT: u16 = 0x1000; // Базовый сегмент загрузки
        const PSP_SEGMENT: u16 = LOAD_SEGMENT - 0x10; // PSP на 256 байт ниже кода
        let mut machine = Motherboard::new(DOS_MEMORY_SIZE);
        let header_size = self.header.e_cparhdr as usize * 16;
        let code_data = &self.data[header_size..];
        let loaded_bytes = code_data.len();
        let bss_bytes = self.header.e_minep as usize * 16;
        let total_prog_bytes = loaded_bytes + bss_bytes;
        let prog_paragraphs = ((total_prog_bytes + 15) / 16) as u16;
        let end_segment = LOAD_SEGMENT + prog_paragraphs;
        Self::create_psp(&mut machine, PSP_SEGMENT, end_segment);
        let code_base = LOAD_SEGMENT as u32 * 16;
        if code_base as usize + code_data.len() > machine.memory.len() {
            return Err("EXE file too large".into());
        }
        for (i, &byte) in code_data.iter().enumerate() {
            machine.write_mem_u8(code_base + i as u32, byte);
        }
        if bss_bytes > 0 {
            let bss_start = code_base + loaded_bytes as u32;
            for i in 0..bss_bytes {
                if (bss_start as usize + i) < machine.memory.len() {
                    machine.write_mem_u8(bss_start + i as u32, 0);
                }
            }
        }

        log::info!(
            "EXE load: file_size={}, header_size={}, code_data_len={}",
            self.data.len(),
            header_size,
            code_data.len(),
        );
        self.apply_relocations(&mut machine, LOAD_SEGMENT);

        let cpu = crate::loader::make_cpu(no_log)?;
        let mut sys = DosSystem::new(machine, cpu, config.clone());

        // Настройка MCB
        sys.first_mcb_segment = PSP_SEGMENT - 1;
        sys.dta_segment = PSP_SEGMENT;
        sys.dta_offset = 0x0080;
        let prog_mcb = MCB {
            signature: MCB_SIGNATURE_NON_LAST,
            owner: PSP_SEGMENT,
            size: 0x10 + prog_paragraphs,
        };
        prog_mcb.write(&mut sys.machine, PSP_SEGMENT - 1);

        let free_mcb = MCB {
            signature: MCB_SIGNATURE_LAST,
            owner: 0,
            size: MAX_MCB_SEGMENT - end_segment - 1,
        };
        free_mcb.write(&mut sys.machine, end_segment);

        let mem_block_size = MAX_MCB_SEGMENT - PSP_SEGMENT;
        sys.write_phys_u16((PSP_SEGMENT as u32 * 16) + 0x02, mem_block_size);

        init_ivt(&mut sys.machine);

        // Инициализация регистров из заголовка EXE
        let cs = LOAD_SEGMENT.wrapping_add(self.header.cs);
        let ip = self.header.ip;
        let ss = LOAD_SEGMENT.wrapping_add(self.header.ss);
        let sp = self.header.sp;

        sys.cpu.registers.set_cs(cs);
        sys.cpu.registers.set_ds(PSP_SEGMENT);
        sys.cpu.registers.set_es(PSP_SEGMENT);
        sys.cpu.registers.set_fs(PSP_SEGMENT);
        sys.cpu.registers.set_ss(ss);
        sys.cpu.registers.set_ip(ip);
        sys.cpu.registers.set_sp(sp);

        // ✅ ИСПРАВЛЕНО: Корректная эмуляция начального PUSH адреса возврата в PSP.
        // Используем штатные функции стека из крейта x86, чтобы избежать ошибок
        // с ручным расчетом физ. адреса и переносом SP через 0.
        stack::push16(&mut sys.cpu, &mut sys.machine, 0x0000); // IP (указывает на INT 20h в PSP)
        stack::push16(&mut sys.cpu, &mut sys.machine, PSP_SEGMENT); // CS

        log::info!(
            "Loaded .EXE file: CS:IP={:04X}:{:04X}, SS:SP={:04X}:{:04X}, DS={:04X}",
            sys.cpu.registers.cs(),
            sys.cpu.registers.ip(),
            sys.cpu.registers.ss(),
            sys.cpu.registers.sp(),
            sys.cpu.registers.ds()
        );

        Ok(sys)
    }

    fn apply_relocations(&self, machine: &mut dyn Machine, load_segment: u16) {
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
            let current = machine.read_mem_u16(fixup_addr);
            let corrected = current.wrapping_add(load_segment);
            machine.write_mem_u16(fixup_addr, corrected);
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

    // ✅ ИСПРАВЛЕНО: Добавлен параметр env_segment
    fn create_psp(machine: &mut Motherboard, segment: u16, env_segment: u16) {
        let psp_base = segment as u32 * 16;
        machine.memory.write_u8(psp_base, 0xCD);
        machine.memory.write_u8(psp_base + 1, 0x20);
        //Начальный размер в памяти - не трогать
        machine.memory.write_u8(psp_base + 2, 0xF0);
        machine.memory.write_u8(psp_base + 3, 0x00);
        //========================================
        machine.memory.write_u8(psp_base + 5, 0xCD);
        machine.memory.write_u8(psp_base + 6, 0x21);
        machine.memory.write_u8(psp_base + 7, 0xCB);
        machine.memory.write_u8(psp_base + 8, 0x00);
        machine.memory.write_u8(psp_base + 9, 0x00);

        // 0x2C: Сегмент окружения (Environment).
        // Теперь указывает на память ПОСЛЕ программы, а не на её код (LOAD_SEGMENT).
        machine.write_mem_u16(psp_base + 0x2C, env_segment);
    }
}
