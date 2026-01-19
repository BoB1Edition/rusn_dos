use std::fs::File;

use log::info;

use crate::{
    DosMachine,
    consts::{DOS_MEMORY_SIZE, SEGMENT_SIZE},
    loader::MzHeader,
    registers::Registers,
};

#[derive(Debug, Clone, Default)]
pub struct DosExecutable {
    header: Option<MzHeader>,
    data: Vec<u8>,
}

impl DosExecutable {
    pub fn from(value: Vec<u8>) -> crate::error::Result<Self> {
        DosExecutable::from_slice(value.as_slice())
    }

    pub fn from_slice(data: &[u8]) -> crate::error::Result<Self> {
        if data.len() < 2 {
            return Err("File too short".into());
        }
        let header = if data.len() >= 64 && u16::from_le_bytes(data[0..2].try_into()?) == 0x5A4D {
            let headers = MzHeader {
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
            };
            Some(headers)
        } else {
            None // это .COM или другой формат
        };
        Ok(Self {
            header: header,
            data: data.to_vec(),
        })
    }

    pub fn relocation(&self, memory: &mut Box<[u8]>) {
        let load_segment = 0x1000;
        if let Some(hdr) = &self.header {
            let reloc_table_offset = hdr.e_lfarlc as usize;
            let reloc_count = hdr.e_relc as usize;

            for i in 0..reloc_count {
                let entry_offset = reloc_table_offset + i * 4;
                if entry_offset + 4 > self.data.len() {
                    break;
                }

                let offset =
                    u16::from_le_bytes([self.data[entry_offset], self.data[entry_offset + 1]]);
                let segment =
                    u16::from_le_bytes([self.data[entry_offset + 2], self.data[entry_offset + 3]]);

                let fixup_addr = (load_segment as u32 + segment as u32) * 16 + offset as u32;
                let fixup_idx = fixup_addr as usize;

                if fixup_idx + 2 > memory.len() {
                    continue;
                }

                let current = u16::from_le_bytes([memory[fixup_idx], memory[fixup_idx + 1]]);
                let corrected = current.wrapping_add(load_segment);

                memory[fixup_idx] = corrected as u8;
                memory[fixup_idx + 1] = (corrected >> 8) as u8;
            }
        }
    }

    pub fn exec(&self) -> Result<crate::DosMachine, Box<dyn std::error::Error>> {
        let mut memory = vec![0u8; DOS_MEMORY_SIZE].into_boxed_slice();
        memory[0x00] = 0xCD; // INT
        memory[0x01] = 0x20; // 20h
        // Указать размер доступной памяти (например, 640 KiB = 0xA000 параграфов)
        let mem_size_para = (DOS_MEMORY_SIZE / 16) as u16;
        memory[0x02] = (mem_size_para & 0xFF) as u8;
        memory[0x03] = ((mem_size_para >> 8) & 0xFF) as u8;
        // INT 21h / RETF для возврата
        memory[0x08] = 0xCD; // INT
        memory[0x09] = 0x21; // 21h
        memory[0x0A] = 0xCB; // RETF
        let mut dos: crate::DosMachine;
        if let Some(hdr) = &self.header {
            //let load_segment = hdr.e_minep;//0x1000;
            let load_segment = 0x1000;
            let header_size = (hdr.e_cparhdr as usize) * 16;

            // Копируем по физическому адресу 0x100
            let load_physical = load_segment as usize * 16;
            if load_physical + self.data.len() > DOS_MEMORY_SIZE {
                return Err("Program too large".into());
            }
            let data = self.data[header_size..].to_vec();
            memory[load_physical..load_physical + data.len()].copy_from_slice(&data);
            self.relocation(&mut memory);
            // Регистры
            let cs = load_segment + hdr.cs; //load_segment + hdr.e_cparhdr;
            let ip = hdr.ip;
            let ss = load_segment + hdr.ss; //load_segment + hdr.e_cparhdr + hdr.e_minep;
            let sp = hdr.sp;

            // PSP
            let code_start_offset = (load_segment as u32 * 16 ) + header_size as u32;
            println!("Loaded .EXE file:");
            println!("  CS:IP = {:#04x}:{:#04x}", cs, ip);
            println!("  SS:SP = {:#04x}:{:#04x}", ss, sp);
            println!(
                "  Code loaded at physical address: {:#x}",
                code_start_offset
            );

            dos = crate::DosMachine {
                memory: memory,
                halted: false,
                registers: Registers::default(),
                logfile: File::create("logopcode_exe.txt")?,
                has_address_size_prefix: false,
                has_operand_size_prefix: false,
            };
            dos.registers.set_cs(cs);
            //dos.registers.ds = load_segment as u16;
            //dos.registers.es = load_segment as u16;
            dos.registers.set_ds(load_segment);
            dos.registers.set_es(load_segment);
            dos.registers.set_ip(ip);
            dos.registers.set_ss(ss);
            dos.registers.set_sp(sp);
        } else {
            info!("Assuming .COM file (starts at 0x100)");
            let com_start = 0x100;
            memory[com_start..com_start + self.data.len()].copy_from_slice(&self.data);
            dos = DosMachine {
                memory: memory,
                halted: false,
                registers: Registers::default(),
                logfile: File::create("logopcode_com.txt")?,
                has_address_size_prefix: false,
                has_operand_size_prefix: false,
            };
            dos.registers.set_cs(0);
            dos.registers.set_ds(dos.registers.cs());
            dos.registers.set_es(dos.registers.cs());
            dos.registers.set_ip(0x0100);
            dos.registers.set_ss(0);
            dos.registers.set_sp(0xFFFE);
            println!("Loaded .COM file:");
            println!("  CS:IP = 0x0000:0x0100");
            println!("  SS:SP = 0x0000:0xFFFE");
            println!("  Code loaded at physical address: 0x100");
        }
        Ok(dos)
    }
}
