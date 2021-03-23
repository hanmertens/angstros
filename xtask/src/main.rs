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
        Some("build") => build::build(&info)?,
        Some("run") => {
            build::build(&info)?;
            run::run(&info)?;
        }
        Some(s) => println!("Unknown subcommand {}", s),
        None => println!("Use subcommand build or run"),
    }
    Ok(())
}
