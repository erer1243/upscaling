use eyre::Result;
use std::{
    fs, io,
    path::{Path, PathBuf},
    str,
};

// Source: https://internals.rust-lang.org/t/create-a-flushing-version-of-print/9870/6
macro_rules! print_flush {
    ($($t:tt)*) => {{
        use ::std::io::Write;
        let mut h = ::std::io::stdout();
        write!(h, $($t)*).unwrap();
        h.flush().unwrap();
    }}
}
pub(crate) use print_flush;

macro_rules! command {
    ($cmd:expr, $($args:expr),* $(,)?) => {{
        let mut cmd = ::std::process::Command::new($cmd);
        $(cmd.arg($args);)*
        cmd
    }};
}
pub(crate) use command;

pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Result<Self> {
        let output = command!("mktemp", "-d", "--suffix=-upscaling").output()?;
        let path_str = str::from_utf8(&output.stdout)?.trim_end();
        Ok(Self(PathBuf::from(path_str)))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        _ = fs::remove_dir_all(self.path());
    }
}

pub fn file_exists<P: AsRef<Path>>(p: P) -> io::Result<bool> {
    p.as_ref().try_exists()
}
