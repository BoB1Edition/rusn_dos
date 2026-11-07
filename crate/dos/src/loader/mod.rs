// Ver: 3 File: crate/dos/src/loader/mod.rs
pub mod com_loader;
pub mod exe_header;
pub mod exe_loader;
pub mod psp;

use crate::system::{DosSystem, ExecRequest};
use emu_config::{EnvironmentConfig, MachineConfig};
use std::path::PathBuf;
use x86::cpu::X86Cpu;

pub enum ExecutableType {
    Com,
    Exe,
}

pub fn detect_executable_type(path: &PathBuf) -> ExecutableType {
    let data = std::fs::read(path).unwrap_or_default();

    if data.len() >= 2
        && u16::from_le_bytes(data[0..2].try_into().unwrap_or([0; 2])) == 0x5A4D
    {
        ExecutableType::Exe
    } else {
        ExecutableType::Com
    }
}

pub(crate) fn make_cpu(no_log: bool) -> Result<X86Cpu, Box<dyn std::error::Error>> {
    if no_log {
        Ok(X86Cpu::new())
    } else {
        Ok(X86Cpu::new_with_tracer("logopcode.txt")?)
    }
}

pub fn load_executable(
    path: PathBuf,
    no_log: bool,
    config: &MachineConfig,
    env: &EnvironmentConfig,
    args: Vec<String>,
) -> Result<DosSystem, Box<dyn std::error::Error>> {
    let mut sys = match detect_executable_type(&path) {
        ExecutableType::Com => {
            let loader = com_loader::ComLoader::from_file(&path)?;
            loader.exec(no_log, config, env, args)?
        }
        ExecutableType::Exe => {
            let mut loader = exe_loader::ExeLoader::from_file(&path)?;
            loader.exec(no_log, config, env, args)?
        }
    };

    sys.init_sysvars();

    sys.process_manager.current_psp = sys.cpu.registers.ds();

    Ok(sys)
}

pub fn load_executable_in_place(
    sys: &mut DosSystem,
    req: &ExecRequest,
    parent_psp: u16,
    tail: &[u8],
    fcb1: &[u8; 16],
    fcb2: &[u8; 16],
) -> Result<u16, u8> {
    match detect_executable_type(&PathBuf::from(&req.path)) {
        ExecutableType::Com => {
            com_loader::load_in_place(sys, req, parent_psp, tail, fcb1, fcb2)
        }
        ExecutableType::Exe => {
            exe_loader::load_in_place(sys, req, parent_psp, tail, fcb1, fcb2)
        }
    }
}

pub fn resolve_child_env_seg(
    sys: &DosSystem,
    parent_psp: u16,
    req_env_seg: u16,
) -> u16 {
    if req_env_seg != 0 {
        return req_env_seg;
    }

    if parent_psp == 0 {
        log::warn!("resolve_child_env_seg: parent_psp is 0, child environment will be 0000");
        return 0;
    }

    sys.read_u16(parent_psp, 0x2C)
}

pub fn clear_psp(sys: &mut DosSystem, psp_segment: u16) {
    for off in 0..0x100u16 {
        sys.write_u8(psp_segment, off, 0);
    }
}

pub fn write_command_tail(sys: &mut DosSystem, psp_segment: u16, tail: &[u8]) {
    let tail_len = tail.len().min(126);

    sys.write_u8(psp_segment, 0x80, tail_len as u8);

    for (i, &b) in tail.iter().take(tail_len).enumerate() {
        sys.write_u8(psp_segment, 0x81u16.wrapping_add(i as u16), b);
    }

    sys.write_u8(
        psp_segment,
        0x81u16.wrapping_add(tail_len as u16),
        0x0D,
    );

    log::debug!(
        "PSP Command Tail at {:04X}:0080: len={}, bytes={:?}",
        psp_segment,
        tail_len,
        &tail[..tail_len]
    );
}

pub fn write_fcb_block(
    sys: &mut DosSystem,
    psp_segment: u16,
    fcb1: &[u8; 16],
    fcb2: &[u8; 16],
) {
    for i in 0..16u16 {
        sys.write_u8(psp_segment, 0x5C + i, fcb1[i as usize]);
        sys.write_u8(psp_segment, 0x6C + i, fcb2[i as usize]);
    }
}

pub fn prepare_child_psp(
    sys: &mut DosSystem,
    psp_segment: u16,
    mem_paragraphs: u16,
    parent_psp: u16,
    req_env_seg: u16,
    tail: &[u8],
    fcb1: &[u8; 16],
    fcb2: &[u8; 16],
) {
    let env_seg = resolve_child_env_seg(sys, parent_psp, req_env_seg);

    clear_psp(sys, psp_segment);

    psp::build_psp(
        &mut sys.machine,
        psp_segment,
        env_seg,
        mem_paragraphs,
        &[],
    );

    write_command_tail(sys, psp_segment, tail);
    write_fcb_block(sys, psp_segment, fcb1, fcb2);
}

pub fn finalize_mcb_owner(sys: &mut DosSystem, psp_segment: u16) {
    let mut mcb = crate::mcb::MCB::read(&sys.machine, psp_segment - 1);
    mcb.owner = psp_segment;
    mcb.write(&mut sys.machine, psp_segment - 1);
}

pub fn setup_child_segments(sys: &mut DosSystem, psp_segment: u16) {
    sys.cpu.registers.set_ds(psp_segment);
    sys.cpu.registers.set_es(psp_segment);
    sys.cpu.registers.set_fs(psp_segment);
    sys.cpu.registers.set_gs(psp_segment);

    sys.dta_segment = psp_segment;
    sys.dta_offset = 0x0080;
}

pub fn push_child_return(sys: &mut DosSystem, psp_segment: u16) {
    sys.push16(psp_segment);
    sys.push16(0x0000);
}

pub fn finalize_child_context(sys: &mut DosSystem, psp_segment: u16) {
    finalize_mcb_owner(sys, psp_segment);
    setup_child_segments(sys, psp_segment);
    push_child_return(sys, psp_segment);
}