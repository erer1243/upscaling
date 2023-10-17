use crate::util::{command, print_flush};
use eyre::{ensure, Result};
use serde::Deserialize;
use std::{
    ffi::OsStr,
    process::{Child, Stdio},
};

const QUIET_ARGS: &[&str] = &["-loglevel", "error"];

pub fn launch_encoder<I, O>(framerate: &str, original_src: I, output_path: O) -> Result<Child>
where
    I: AsRef<OsStr>,
    O: AsRef<OsStr>,
{
    let mut cmd = command! {
        "ffmpeg",
            "-f", "image2pipe",
            "-framerate", framerate,
            "-i", "-",
            "-i", original_src,
            "-map", "0:v:0", "-map", "1:a:0?",
            "-c:a", "copy", "-c:v", "libx265",
            "-x265-params", "log-level=none",
            output_path
    };
    cmd.args(QUIET_ARGS);
    cmd.stdin(Stdio::piped());
    Ok(cmd.spawn()?)
}

pub fn launch_decoder<S: AsRef<OsStr>>(source_video: S) -> Result<Child> {
    let mut cmd = command! {
        "ffmpeg",
            "-i", source_video,
            "-c:v", "png",
            "-f", "image2pipe",
            "-"
    };
    cmd.args(QUIET_ARGS);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    Ok(cmd.spawn()?)
}

pub fn probe_video<P: AsRef<OsStr>>(path: P) -> Result<StreamData> {
    // Run ffprobe on video
    print_flush!("Probing video... ");
    let mut cmd = command! {
        "ffprobe",
            "-count_frames",
            "-select_streams", "v:0",
            "-show_streams",
            "-print_format", "json",
            path
    };
    cmd.args(QUIET_ARGS);
    let output = cmd.output()?;
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
    ensure!(framerate == avg_fr, "variable framerate input");

    Ok(StreamData {
        framerate,
        frames: frames_str.parse()?,
    })
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
