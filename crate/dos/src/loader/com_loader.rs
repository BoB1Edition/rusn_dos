// Ver: 3 File: crate/dos/src/loader/com_loader.rs
use bus::{Machine, Motherboard};
use emu_config::{EnvironmentConfig, MachineConfig};

use crate::consts::{DOS_MEMORY_SIZE, MAX_MCB_SEGMENT, MCB_SIGNATURE_LAST, MCB_SIGNATURE_NON_LAST};
use crate::ivt::init_ivt;
use crate::mcb::MCB;
use crate::system::DosSystem;

#[derive(Debug, Clone, Default)]
pub struct ComLoader {
    data: Vec<u8>,
}

impl ComLoader {
    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self { data })
    }

    pub fn exec(
        self,
        no_log: bool,
        config: &MachineConfig,
        env: &EnvironmentConfig,
        args: Vec<String>,
    ) -> Result<DosSystem, Box<dyn std::error::Error>> {
        const PSP_SEGMENT: u16 = 0x1000;
        const ENV_SEGMENT: u16 = PSP_SEGMENT - 0x10;
        const CODE_OFFSET: u16 = 0x0100;
        const STACK_TOP: u16 = 0xFFFE;
        let mut machine = Motherboard::new(DOS_MEMORY_SIZE);
        let env_segment =
            crate::loader::psp::build_environment_block(&mut machine, PSP_SEGMENT, env);

        let code_base = (PSP_SEGMENT as u32 * 16 + CODE_OFFSET as u32) as usize;
        if code_base + self.data.len() > machine.memory.len() {
            return Err("COM file too large (>64KB)".into());
        }
        for (i, &byte) in self.data.iter().enumerate() {
            machine.write_mem_u8((code_base + i) as u32, byte);
        }

        let com_paragraphs = ((self.data.len() + 0x100 + 15) / 16) as u16;
        let end_segment = PSP_SEGMENT + com_paragraphs;
        let mem_paragraphs = MAX_MCB_SEGMENT - PSP_SEGMENT;

        crate::loader::psp::build_psp(
            &mut machine,
            PSP_SEGMENT,
            env_segment,
            mem_paragraphs,
            &args,
        );

        let prog_mcb = MCB {
            signature: MCB_SIGNATURE_NON_LAST,
            owner: PSP_SEGMENT,
            size: com_paragraphs,
        };
        let cpu = crate::loader::make_cpu(no_log)?;
        let mut sys = DosSystem::new(machine, cpu, config.clone(), env.clone());
        sys.first_mcb_segment = PSP_SEGMENT - 1;
        sys.dta_segment = PSP_SEGMENT;
        sys.dta_offset = 0x0080;

        prog_mcb.write(&mut sys.machine, PSP_SEGMENT - 1);
        let free_mcb = MCB {
            signature: MCB_SIGNATURE_LAST,
            owner: 0,
            size: MAX_MCB_SEGMENT - end_segment - 1,
        };
        free_mcb.write(&mut sys.machine, end_segment);

        init_ivt(&mut sys.machine);

        sys.cpu.registers.set_cs(PSP_SEGMENT);
        sys.cpu.registers.set_ds(PSP_SEGMENT);
        sys.cpu.registers.set_es(PSP_SEGMENT);
        sys.cpu.registers.set_ss(PSP_SEGMENT);
        sys.cpu.registers.set_ip(CODE_OFFSET);
        sys.cpu.registers.set_fs(PSP_SEGMENT);
        sys.cpu.registers.set_gs(PSP_SEGMENT);
        sys.cpu.registers.set_sp(STACK_TOP);

        log::info!(
            "Loaded .COM file: CS:IP={:04X}:{:04X}, SS:SP={:04X}:{:04X}, DS={:04X}",
            sys.cpu.registers.cs(),
            sys.cpu.registers.ip(),
            sys.cpu.registers.ss(),
            sys.cpu.registers.sp(),
            sys.cpu.registers.ds()
        );
        Ok(sys)
    }
}
