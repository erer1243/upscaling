use eyre::Result;
use std::{
    fmt::Write,
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

pub fn progress_bar(width: usize, value: usize, max: usize) -> String {
    let mut s = String::with_capacity(width);
    let iw = width - 2;
    let n = iw * value / max;
    s.push('[');
    (0..n).for_each(|_| s.push('#'));
    (n..iw).for_each(|_| s.push('.'));
    s.push(']');
    s
}

pub fn pretty_time(total_secs: u64) -> String {
    let hrs = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    let mut s = String::new();
    if hrs > 0 {
        write!(s, "{hrs}h").unwrap();
    }
    if mins > 0 {
        write!(s, "{mins}m").unwrap();
    }
    write!(s, "{secs}s").unwrap();
    s
}

pub fn clear_line() {
    print_flush!("\r{: <100}\r", "");
}
