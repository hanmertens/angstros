use anyhow::Result;
use config::BuildInfo;
use std::path::Path;

mod build;
mod command;
mod config;
mod run;

fn main() -> Result<()> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_dir = manifest.ancestors().nth(1).unwrap();
    let info = BuildInfo::new(base_dir);

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("build") => {
            build::build(&info, false)?;
        }
        Some("debug") => {
            let info = build::build(&info, false)?;
            run::debug(&info)?;
        }
        Some("run") => {
            let info = build::build(&info, false)?;
            run::run(&info)?;
        }
        Some("test") => {
            let info = build::build(&info, true)?;
            run::test(&info)?;
        }
        Some(s) => println!("Unknown subcommand {}", s),
        None => println!("Use subcommand build, debug, run or test"),
    }
    Ok(())
}
