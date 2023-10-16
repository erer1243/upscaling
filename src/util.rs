use eyre::{bail, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    str,
};

pub fn ensure_command(cmd: &mut Command) -> Result<Output> {
    fn pretty_command(cmd: &Command) -> String {
        shlex::join(
            std::iter::once(cmd.get_program().to_str().unwrap())
                .chain(cmd.get_args().map(|oss| oss.to_str().unwrap())),
        )
    }

    let output = cmd
        .output()
        .with_context(|| format!("command: {}", pretty_command(cmd)))?;

    if output.status.success() {
        Ok(output)
    } else {
        let program = cmd.get_program().to_str().unwrap();
        let full_command = pretty_command(&cmd);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{program} failed\ncommand:\n{full_command}\n\nstderr:\n{stderr}")
    }
}

// Source: https://internals.rust-lang.org/t/create-a-flushing-version-of-print/9870/6
macro_rules! print_flush {
    ( $($t:tt)* ) => {
        {{
            use ::std::io::Write;
            let mut h = ::std::io::stdout();
            write!(h, $($t)* ).unwrap();
            h.flush().unwrap();
        }}
    }
}
pub(crate) use print_flush;

pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Result<Self> {
        let mut cmd = Command::new("mktemp");
        cmd.args(["-d", "--suffix=-upscaling"]);
        let output = ensure_command(&mut cmd)?;
        let path_str = str::from_utf8(&output.stdout)?.trim_end();
        let p = PathBuf::from(path_str);
        Ok(Self(p))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.path());
    }
}
