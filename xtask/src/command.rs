use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::{
    env,
    ffi::OsStr,
    io,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Output, Stdio},
    str,
};

pub trait CommandResultExt {
    type T;

    fn check_status(self, name: &str) -> Result<Self::T>;
}

impl CommandResultExt for io::Result<ExitStatus> {
    type T = ();

    fn check_status(self, name: &str) -> Result<()> {
        let exit_code = self.with_context(|| format!("{} could not be executed", name))?;
        if exit_code.success() {
            Ok(())
        } else {
            Err(exit_code
                .code()
                .map_or(anyhow!("{} terminated by signal", name), |code| {
                    anyhow!("{} exited with status code {}", name, code)
                }))
        }
    }
}

impl CommandResultExt for io::Result<Output> {
    type T = Output;

    fn check_status(self, name: &str) -> Result<Output> {
        let output = self.with_context(|| format!("{} could not be executed", name))?;
        Ok(output.status).check_status(name)?;
        Ok(output)
    }
}

impl CommandResultExt for io::Result<Child> {
    type T = Child;

    fn check_status(self, name: &str) -> Result<Child> {
        let output = self.with_context(|| format!("{} could not be executed", name))?;
        Ok(output)
    }
}

pub struct Cargo(Command);

impl Cargo {
    fn new() -> Self {
        Self(env::var_os("CARGO").map_or_else(|| Command::new(env!("CARGO")), Command::new))
    }

    fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.0.arg(arg);
        self
    }

    pub fn build() -> Self {
        let mut c = Self::new();
        c.arg("build")
            .arg("--message-format=json-render-diagnostics");
        c.0.stderr(Stdio::inherit());
        c
    }

    pub fn package<S: AsRef<OsStr>>(&mut self, package: S) -> &mut Self {
        self.arg("--package").arg(package)
    }

    pub fn z<S: AsRef<OsStr>>(&mut self, unstable: S) -> &mut Self {
        self.arg("-Z").arg(unstable)
    }

    pub fn target<S: AsRef<OsStr>>(&mut self, target: S) -> &mut Self {
        self.arg("--target").arg(target)
    }

    pub fn env<K: AsRef<OsStr>, V: AsRef<OsStr>>(&mut self, key: K, val: V) -> &mut Self {
        self.0.env(key, val);
        self
    }

    fn output(&mut self) -> Result<Output> {
        self.0.output().check_status("Cargo")
    }

    fn executables(&mut self) -> Result<Vec<PathBuf>> {
        let cmd = self.output()?;

        let invalid = "Invalid Cargo output";
        let mut executables = Vec::new();
        for line in str::from_utf8(&cmd.stdout).context(invalid)?.lines() {
            let cargo: CargoOutput = serde_json::from_str(line).context(invalid)?;
            if let Some(x) = cargo.executable {
                executables.push(x);
            }
        }
        Ok(executables)
    }

    pub fn single_executable(&mut self) -> Result<PathBuf> {
        let mut vec = self.executables()?;
        match vec.len() {
            1 => Ok(vec.remove(0)),
            n => Err(anyhow!("Unexpected number of executables {}", n)),
        }
    }
}

#[derive(Deserialize)]
struct CargoOutput {
    executable: Option<PathBuf>,
}
