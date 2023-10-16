use crate::util::{ensure_command, print_flush};
use eyre::{bail, Result, WrapErr};
use lazy_format::lazy_format as lformat;
use once_cell::sync::Lazy;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};
use zip::unstable::stream::ZipStreamReader;

pub static EXECUTABLE: Lazy<PathBuf> = Lazy::new(|| {
    dirs::cache_dir()
        .expect("user cache dir")
        .join("realesrgan/realesrgan-ncnn-vulkan")
});

pub fn check_and_download() -> Result<()> {
    const URL: &str = "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-ubuntu.zip";

    if EXECUTABLE.exists() {
        return Ok(());
    }

    let dir = EXECUTABLE.parent().unwrap();
    println!("Downloading Real-ESRGAN");
    println!("  from {URL:?}");
    println!("  into {dir:?}");

    print_flush!("Downloading... ");

    fs::create_dir_all(dir).context(lformat!("mkdir {dir:?}"))?;

    let resp = ureq::get(URL).call()?;
    let status = resp.status();
    if status != 200 {
        bail!("status code {status}");
    }

    ZipStreamReader::new(resp.into_reader())
        .extract(dir)
        .context("failed unzipping stream")?;

    if !EXECUTABLE.exists() {
        bail!("Download succeeded but file doesn't exist: {EXECUTABLE:?}");
    }

    fs::set_permissions(&*EXECUTABLE, fs::Permissions::from_mode(0o744))?;

    println!("done");

    Ok(())
}

pub fn upscale_images_in_dir(input: &str, output: &str, scale: &str) -> Result<()> {
    let mut cmd = Command::new(&*EXECUTABLE);
    cmd.args(["-s", scale, "-i", input, "-o", output]);
    ensure_command(&mut cmd)?;
    Ok(())
}
