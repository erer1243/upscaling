use crate::util::print_flush;
use eyre::{ensure, Result};
use serde::Deserialize;
use std::{
    ffi::OsStr,
    process::{Child, Command, Stdio},
};

const BASIC_ARGS: &[&str] = &["-loglevel", "error"];

pub fn launch_encoder<I, O>(framerate: &str, original_src: I, output_path: O) -> Result<Child>
where
    I: AsRef<OsStr>,
    O: AsRef<OsStr>,
{
    let mut cmd = Command::new("ffmpeg");
    cmd.args(BASIC_ARGS);
    cmd.args(["-f", "image2pipe", "-framerate", framerate, "-i", "-"]);
    cmd.args(["-i".as_ref(), original_src.as_ref()]);
    cmd.args(["-map", "0:v:0", "-map", "1:a:0?"]);
    cmd.args([
        "-c:a",
        "copy",
        "-c:v",
        "libx265",
        "-x265-params",
        "log-level=none",
    ]);
    cmd.arg(output_path);
    cmd.stdin(Stdio::piped());
    Ok(cmd.spawn()?)
}

pub fn launch_decoder<S: AsRef<OsStr>>(source_video: S) -> Result<Child> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(BASIC_ARGS);
    cmd.args(["-i".as_ref(), source_video.as_ref()]);
    cmd.args(["-c:v", "png", "-f", "image2pipe", "-"]);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    Ok(cmd.spawn()?)
}

pub fn probe_video<P: AsRef<OsStr>>(path: P) -> Result<StreamData> {
    // Run ffprobe on video
    print_flush!("Probing video... ");
    let mut cmd = Command::new("ffprobe");
    cmd.args(BASIC_ARGS);
    cmd.args([
        "-count_frames",
        "-select_streams",
        "v:0",
        "-show_streams",
        "-print_format",
        "json",
    ]);
    cmd.arg(path);
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
