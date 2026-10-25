// Ver: 5 File: ./src/main.rs
use clap::{Parser, Subcommand};
use log::info;
use std::{env, error::Error, path::PathBuf};
mod app;
mod stdin_bridge;

#[derive(Debug, Parser)]
#[command(name = "rusn_dos")]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
    #[arg(long, default_value = "false")]
    no_log: bool,
    #[arg(short, long, default_value = "false")]
    graphics: bool,
    #[arg(long, default_value = "false")]
    no_stdin_keys: bool,
    #[arg(long)]
    mount: Vec<String>,
    #[arg(long, default_value = "20")]
    max_call_depth: u16,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Win {
        #[arg(short, long, default_value = "./")]
        workdir: PathBuf,
    },
    #[command(disable_help_flag = true)]
    Run {
        program: PathBuf,
        #[arg(short = 's', long = "startdir", default_value = "./")]
        startdir: PathBuf,
        #[arg(short = 'w', long = "workdir", default_value = "./")]
        workdir: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        program_args: Vec<String>,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let cli = Cli::parse();
    if let Some(command) = cli.command {
        match command {
            Commands::Win { workdir } => {
                let _ = env::set_current_dir(&workdir);
                run_window(cli.config)?;
            }
            Commands::Run {
                program,
                startdir,
                workdir,
                program_args,
            } => {
                let actual_program = if program.exists() && program.is_file() {
                    program
                } else {
                    let joined = startdir.join(&program);
                    if joined.exists() && joined.is_file() {
                        joined
                    } else {
                        program
                    }
                };
                run_program(
                    actual_program,
                    startdir,
                    workdir,
                    cli.config,
                    cli.no_log,
                    cli.graphics,
                    cli.no_stdin_keys,
                    program_args,
                )?;
            }
        }
    } else {
        run_window(cli.config)?;
    }
    Ok(())
}

fn run_program(
    program: PathBuf,
    startdir: PathBuf,
    workdir: PathBuf,
    config: PathBuf,
    no_log: bool,
    graphics: bool,
    no_stdin_keys: bool,
    program_args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let app = app::App::load_from_file(config);
    if graphics {
        log::debug!("graphics");
        app.run_with_graphics(program, startdir, workdir, no_log, program_args)?;
    } else {
        app.run(
            program,
            startdir,
            workdir,
            no_log,
            no_stdin_keys,
            program_args,
        )?;
    }
    Ok(())
}

fn run_window(config: PathBuf) -> Result<(), Box<dyn Error>> {
    todo!("run_window");
    Ok(())
}
