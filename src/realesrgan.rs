use crate::util::{command, file_exists, print_flush};
use eyre::{ensure, Result};
use once_cell::sync::Lazy;
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Stdio};
use zip::unstable::stream::ZipStreamReader;

pub static EXECUTABLE: Lazy<PathBuf> = Lazy::new(|| {
    dirs::cache_dir()
        .expect("user cache dir")
        .join("realesrgan/realesrgan-ncnn-vulkan")
});

pub fn check_and_download() -> Result<()> {
    const URL: &str = "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-ubuntu.zip";
    let exec = &*EXECUTABLE;

    if file_exists(exec)? {
        return Ok(());
    }

    let dir = exec.parent().unwrap();
    fs::create_dir_all(dir)?;

    println!("Downloading Real-ESRGAN");
    println!("  from {URL:?}");
    println!("  into {dir:?}");
    print_flush!("Downloading... ");

    let resp = ureq::get(URL).call()?;
    let status = resp.status();
    ensure!(status == 200, "status code {status}");
    ZipStreamReader::new(resp.into_reader()).extract(dir)?;
    ensure!(file_exists(exec)?, "unknown failure");
    fs::set_permissions(exec, fs::Permissions::from_mode(0o744))?;

    println!("done");
    Ok(())
}

pub fn upscale_images_in_dir<I, O, S, M>(input: I, output: O, scale: S, model: M) -> Result<()>
where
    I: AsRef<OsStr>,
    O: AsRef<OsStr>,
    S: AsRef<OsStr>,
    M: AsRef<OsStr>,
{
    let mut cmd = command! {
        &*EXECUTABLE,
            "-s", scale,
            "-i", input,
            "-o", output,
            "-n", model,
    };
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd.status()?;
    Ok(())
}
