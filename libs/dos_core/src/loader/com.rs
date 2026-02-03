use std::fs::File;

use log::info;

use crate::{DosMachine, consts::DOS_MEMORY_SIZE, loader::MzHeader, registers::Registers};

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

    pub fn relocation(&self, memory: &mut Box<[u8]>, load_segment: u16) {
        if let Some(hdr) = &self.header {
            let reloc_table_offset = (hdr.e_lfarlc as usize);// * 16;
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
                let fixup_addr = ((load_segment as u32 + segment as u32)<<4) + offset as u32;
                let idx = fixup_addr as usize;

                if idx + 2 <= memory.len() {
                    let current = u16::from_le_bytes([memory[idx], memory[idx + 1]]);
                    let corrected = current.wrapping_add(load_segment);
                    memory[idx] = corrected as u8;
                    memory[idx + 1] = (corrected >> 8) as u8;
                }
            }
        }
    }

    fn create_psp(&self, load_segment: usize, memory: &mut Box<[u8]>) {
        let psp_base = (load_segment - 0x10)<<4;
        memory[psp_base] = 0xCD;
        memory[psp_base + 1] = 0x20;
        let mem_size_para = (DOS_MEMORY_SIZE / 16 - load_segment as usize) as u16;
        memory[psp_base + 2] = (mem_size_para & 0xFF) as u8;
        memory[psp_base + 3] = ((mem_size_para >> 8) & 0xFF) as u8;
        memory[psp_base + 8] = 0xCD;
        memory[psp_base + 9] = 0x21;
        memory[psp_base + 10] = 0xCB;
    }

    pub fn exec(&self) -> Result<crate::DosMachine, Box<dyn std::error::Error>> {
        let mut memory = vec![0u8; DOS_MEMORY_SIZE].into_boxed_slice();
        
        if let Some(hdr) = &self.header {
            const LOAD_SEGMENT: usize = 0x1000;
            self.create_psp(LOAD_SEGMENT, &mut memory);

            let header_size = (hdr.e_cparhdr as usize) * 16;
            let code_data = &self.data[header_size..];
            let code_base = LOAD_SEGMENT << 4;
            if code_base as usize + code_data.len() > DOS_MEMORY_SIZE {
                return Err("Program too large".into());
            }
            memory[code_base..code_base + code_data.len()].copy_from_slice(code_data);
            /*for b in &memory[code_base..code_base + code_data.len()] {
                println!("byte: {:#02x}", b);
            }*/
            self.relocation(&mut memory, LOAD_SEGMENT as u16);
            /*for b in &memory[code_base..code_base + code_data.len()] {
                println!("post reloc byte: {:#02x}", b);
            }*/
            let cs = (LOAD_SEGMENT as u16).wrapping_add(hdr.cs);
            let ip = hdr.ip;
            let ss = (LOAD_SEGMENT as u16).wrapping_add(hdr.ss);
            let sp = hdr.sp;

            println!("Loaded .EXE file:");
            println!("  CS:IP = {:#04x}:{:#04x}", cs, ip);
            println!("  SS:SP = {:#04x}:{:#04x}", ss, sp);
            println!("  DS = {:#04x}", LOAD_SEGMENT - 0x10);

            let mut dos = crate::DosMachine {
                memory,
                halted: false,
                registers: Registers::default(),
                logfile: File::create("logopcode_exe.txt")?,
                has_address_size_prefix: false,
                has_operand_size_prefix: false,
                has_extended_prefix: false,
                override_segment: None,
                opcode_override_segment: None,
            };
            dos.registers.set_cs(cs);
            dos.registers.set_ds((LOAD_SEGMENT - 0x10)as u16);
            dos.registers.set_es((LOAD_SEGMENT - 0x10) as u16);
            dos.registers.set_ip(ip);
            dos.registers.set_ss(ss);
            dos.registers.set_sp(sp);

            Ok(dos)
        } else {
            info!("Assuming .COM file (starts at 0x100)");
            const LOAD_SEGMENT: usize = 0x10;
            let com_start= LOAD_SEGMENT<<4;
            memory[com_start..com_start + self.data.len()].copy_from_slice(&self.data);
            self.create_psp(LOAD_SEGMENT, &mut memory);
            let mut dos = DosMachine {
                memory,
                halted: false,
                registers: Registers::default(),
                logfile: File::create("logopcode_com.txt")?,
                has_address_size_prefix: false,
                has_operand_size_prefix: false,
                has_extended_prefix: false,
                override_segment: None,
                opcode_override_segment: None,
            };
            dos.registers.set_cs(0);
            dos.registers.set_ds(0);
            dos.registers.set_es(0);
            dos.registers.set_ip(0x0100);
            dos.registers.set_ss(0);
            dos.registers.set_sp(0xFFFE);
            println!("Loaded .COM file:");
            println!("  CS:IP = 0x0000:0x0100");
            println!("  SS:SP = 0x0000:0xFFFE");
            println!("  Code loaded at physical address: 0x100");
            Ok(dos)
        }
    }
}
