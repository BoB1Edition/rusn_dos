// Ver: 7 File: crate/dos/src/loader/com_loader.rs
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

pub fn load_in_place(
    sys: &mut crate::system::DosSystem,
    req: &crate::system::ExecRequest,
    tail: &[u8],
    fcb1: &[u8; 16],
    fcb2: &[u8; 16],
) -> Result<u16, u8> {
    let data = std::fs::read(&req.path).map_err(|_| 0x02u8)?;
    if data.len() + 0x100 > 0x10000 {
        return Err(0x08); // Insufficient memory
    }
    let min_paragraphs = ((data.len() + 0x100 + 15) / 16) as u16;
    let available = crate::mcb::max_available(&sys.machine, sys.first_mcb_segment);
    if available < min_paragraphs {
        return Err(0x08);
    }

    let alloc_paragraphs = available.min(0x1000);
    let parent_psp = sys.cpu.registers.ds();
    let parent_env_seg = sys.read_u16(parent_psp, 0x2C);

    let actual_env_seg = if req.env_seg == 0 {
        parent_env_seg
    } else {
        req.env_seg
    };

    let psp_segment =
        crate::mcb::allocate(&mut sys.machine, sys.first_mcb_segment, alloc_paragraphs)
            .ok_or(0x08)?;

    // Очищаем PSP перед заполнением.
    for off in 0..0x100u16 {
        sys.write_u8(psp_segment, off, 0);
    }

    crate::loader::psp::build_psp(
        &mut sys.machine,
        psp_segment,
        actual_env_seg,
        alloc_paragraphs,
        &[],
    );

    // Command tail: максимум 126 байт.
    let tail_len = tail.len().min(126);
    sys.write_u8(psp_segment, 0x80, tail_len as u8);
    for (i, &b) in tail.iter().take(tail_len).enumerate() {
        sys.write_u8(psp_segment, 0x81u16.wrapping_add(i as u16), b);
    }
    sys.write_u8(psp_segment, 0x81u16.wrapping_add(tail_len as u16), 0x0D);

    // FCB1 и FCB2.
    for i in 0..16u16 {
        sys.write_u8(psp_segment, 0x5C + i, fcb1[i as usize]);
        sys.write_u8(psp_segment, 0x6C + i, fcb2[i as usize]);
    }

    // Загружаем код начиная с PSP:0100.
    let code_base = ((psp_segment as u32) << 4) + 0x100;
    for (i, &byte) in data.iter().enumerate() {
        sys.write_phys_u8(code_base + i as u32, byte);
    }

    // Обновляем владельца MCB.
    let mut mcb = crate::mcb::MCB::read(&sys.machine, psp_segment - 1);
    mcb.owner = psp_segment;
    mcb.write(&mut sys.machine, psp_segment - 1);

    // Регистры процесса.
    sys.cpu.registers.set_cs(psp_segment);
    sys.cpu.registers.set_ip(0x0100);
    sys.cpu.registers.set_ss(psp_segment);

    let sp = if alloc_paragraphs >= 0x1000 {
        0xFFFE
    } else {
        (alloc_paragraphs as u32 * 16).wrapping_sub(2) as u16
    };
    sys.cpu.registers.set_sp(sp);

    sys.cpu.registers.set_ds(psp_segment);
    sys.cpu.registers.set_es(psp_segment);
    sys.cpu.registers.set_fs(psp_segment);
    sys.cpu.registers.set_gs(psp_segment);
    sys.dta_segment = psp_segment;
    sys.dta_offset = 0x0080;
    sys.push16(psp_segment);
    sys.push16(0x0000);

    log::info!(
        "Loaded .COM in-place: CS:IP={:04X}:0100, SS:SP={:04X}:{:04X}, DS={:04X}, alloc={:04X}",
        psp_segment,
        psp_segment,
        sys.cpu.registers.sp(),
        psp_segment,
        alloc_paragraphs
    );

    Ok(psp_segment)
}
