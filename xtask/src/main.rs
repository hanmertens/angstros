use anyhow::Result;
use clap::Clap;
use config::{Info, SubCommand};

mod build;
mod command;
mod config;
mod run;

fn main() -> Result<()> {
    let info = Info::parse();
    match info.cmd {
        SubCommand::Build => {
            build::build(&info)?;
        }
        SubCommand::Debug => {
            let info = build::build(&info)?;
            run::debug(&info)?;
        }
        SubCommand::Run => {
            let info = build::build(&info)?;
            run::run(&info)?;
        }
        SubCommand::Test => {
            let info = build::build(&info)?;
            run::test(&info)?;
        }
    }
    Ok(())
}
