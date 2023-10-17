use eyre::Result;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
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

pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Result<Self> {
        let mut cmd = Command::new("mktemp");
        cmd.args(["-d", "--suffix=-upscaling"]);
        let output = cmd.output()?;
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

pub fn file_exists<P: AsRef<Path>>(p: P) -> io::Result<bool> {
    p.as_ref().try_exists()
}
