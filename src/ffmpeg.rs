use crate::util::{command, print_flush, TempDir};
use eyre::{ensure, Result};
use serde::Deserialize;
use std::{
    ffi::OsStr,
    path::PathBuf,
    process::{Child, Stdio},
};

pub fn launch_encoder<I, O>(framerate: &str, original_src: I, output_path: O) -> Result<Child>
where
    I: AsRef<OsStr>,
    O: AsRef<OsStr>,
{
    let mut cmd = command! {
        "ffmpeg",
            "-loglevel", "error",
            "-f", "image2pipe",
            "-framerate", framerate,
            "-i", "-",
            "-i", original_src,
            "-map", "0:v:0", "-map", "1:a:0?",
            "-c:a", "copy", "-c:v", "libx265",
            "-x265-params", "log-level=none",
            output_path
    };
    cmd.stdin(Stdio::piped());
    Ok(cmd.spawn()?)
}

pub fn launch_decoder<S: AsRef<OsStr>>(source_video: S) -> Result<Child> {
    let mut cmd = command! {
        "ffmpeg",
            "-loglevel", "error",
            "-i", source_video,
            "-c:v", "png",
            "-f", "image2pipe",
            "-"
    };
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    Ok(cmd.spawn()?)
}

pub fn probe_video<P: AsRef<OsStr>>(path: P) -> Result<Result<StreamData, VFRError>> {
    // Run ffprobe on video
    print_flush!("Probing video... ");
    let output = command! {
        "ffprobe",
            "-count_frames",
            "-select_streams", "v:0",
            "-show_streams",
            "-print_format", "json",
            path
    }
    .output()?;
    println!("done");

    // Deserialize ffprobe data
    let ProbeData { mut streams } = serde_json::from_slice::<ProbeData>(&output.stdout)?;

    // Sanity checks on deserialized stream data
    let n_streams = streams.len();
    ensure!(n_streams == 1, "{n_streams} video streams in file");
    let StreamDataExt {
        r_frame_rate: framerate,
        avg_frame_rate: avg_fr,
        nb_read_frames: frames_str,
    } = streams.pop().unwrap();

    let inner_res = if framerate == avg_fr {
        Ok(StreamData {
            framerate,
            frames: frames_str.parse()?,
        })
    } else {
        Err(VFRError)
    };

    Ok(inner_res)
}

#[derive(Deserialize)]
struct ProbeData {
    streams: Vec<StreamDataExt>,
}

#[derive(Deserialize)]
struct StreamDataExt {
    r_frame_rate: String,
    avg_frame_rate: String,
    nb_read_frames: String,
}

#[derive(Debug)]
pub struct StreamData {
    // ffmpeg/ffprobe use strings of fractions for precise framerate, eg "30/1" for 30fps
    pub framerate: String,
    pub frames: u64,
}

// Variable framerate
pub struct VFRError;

pub fn convert_vfr_to_cfr<P: AsRef<OsStr>>(video: P) -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let output = temp_dir.path().join("converted_to_cfr.mp4");
    let mut cmd = command! {
        "ffmpeg",
            "-loglevel", "error",
            "-i", video,
            // "-c:v", "copy",
            "-c:a", "copy",
            "-vsync", "cfr",
            &output
    };
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    let status = cmd.status()?;
    ensure!(status.success(), "conversion to cfr failed");
    Ok((temp_dir, output))
}
