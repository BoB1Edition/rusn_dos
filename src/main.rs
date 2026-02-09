// Ver: 1
use clap::{Parser, Subcommand};
use log::info;
use std::{env, error::Error, path::PathBuf};

mod app;

#[derive(Debug, Parser)]
#[command(name = "rusn_dos")]
#[command(version, about)]
struct Cli {
    /// mode run program
    #[command(subcommand)]
    command: Option<Commands>,
    /// optional config path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Win {
        #[arg(short, long, default_value = "./")]
        workdir: PathBuf,
    },
    Run {
        program: PathBuf,
        #[arg(short, long, default_value = "./")]
        workdir: PathBuf,
    },
}

fn set_workdir(workdir: &PathBuf) -> Result<(), Box<dyn Error>> {
    env::set_current_dir(workdir)
        .map_err(|e| e.into())
        .map(|_| println!("Current directory: {}", workdir.display()))
}

fn main() -> Result<(), Box<dyn Error>>{
    env_logger::init();
    let cli = Cli::parse();
    if let Some(command) = cli.command {
        match command {
            Commands::Win { workdir } => {
                let _ = set_workdir(&workdir);
                run_window(cli.config)?;
            }
            Commands::Run { program, workdir } => {
                let _ = set_workdir(&workdir);
                if program.exists() && program.is_file() {
                    run_program(program, cli.config)?;
                } else {
                    info!("program not found or program not a file");
                }
            }
        }
    } else {
        run_window(cli.config)?;
    }
    Ok(())
}

fn run_program(program: PathBuf, config: PathBuf) -> Result<(), Box<dyn Error>> {
    let app = app::App::load_from_file(config);
    app.run(program)?;

    Ok(())
}

fn run_window(config: PathBuf) -> Result<(), Box<dyn Error>> {
    todo!()
}
