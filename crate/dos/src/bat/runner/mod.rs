// Ver: 1 File: crate/dos/src/bat/runner/mod.rs

mod commands;
mod exec;
mod flow;
mod resolve;
mod types;
mod util;

pub use types::{BatConfig, PumpFn, ShellState};

use std::collections::HashMap;
use std::path::PathBuf;

use bus::Display;
use emu_config::MachineConfig;

use crate::filesystem::DiskDriver;

/// Публичный API не изменился: `run_bat` — точка входа BAT-оболочки.
#[allow(clippy::too_many_arguments)]
pub fn run_bat(
    host_path: PathBuf,
    args: Vec<String>,
    drivers: Vec<DiskDriver>,
    machine_cfg: MachineConfig,
    env: emu_config::EnvironmentConfig,
    init_drive: char,
    init_dir: String,
    cfg: BatConfig,
    display: &mut Option<&mut dyn Display>,
    pump: &mut PumpFn,
) -> Result<u8, Box<dyn std::error::Error>> {
    let mut state = ShellState {
        cwd: HashMap::new(),
        current_drive: init_drive,
        env: env.variables.clone(),
        echo_on: true,
        errorlevel: 0,
    };
    state
        .cwd
        .insert(init_drive, init_dir.trim_matches(['\\', '/']).to_string());
    let name = host_path
        .file_name()
        .map(|n| n.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    let mut params = vec![name];
    params.extend(args.iter().cloned());
    let mut r = Runner {
        state,
        drivers,
        machine_cfg,
        cfg,
        params: Vec::new(),
        depth: 0,
        pending_cls: false,
    };
    let code = r.run_batch(&host_path, params, 1, display, pump)?;
    Ok(code)
}

struct Runner {
    state: ShellState,
    drivers: Vec<DiskDriver>,
    machine_cfg: MachineConfig,
    cfg: BatConfig,
    params: Vec<String>,
    depth: u16,
    pending_cls: bool,
}
