use crate::util::{ensure_command, print_flush};
use eyre::{bail, Result};
use serde::Deserialize;
use std::process::{Child, Command, Stdio};

const BASIC_ARGS: &[&str] = &["-loglevel", "error"];

pub fn launch_encoder(framerate: &str, original_src: &str, output_path: &str) -> Result<Child> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(BASIC_ARGS);
    cmd.args([
        // Frames input through stdin
        "-f",
        "image2pipe",
        "-framerate",
        framerate,
        "-i",
        "-",
        // Audio input from original video file
        "-i",
        original_src,
        // Select input streams (video0 & audio1) to for output
        "-map",
        "0:v:0",
        "-map",
        "1:a:0?",
        // Copy audio exactly
        "-c:a",
        "copy",
        // Encode with hevc at given framerate
        "-c:v",
        "libx265",
        "-x265-params",
        "log-level=none",
        output_path,
    ]);

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    // cmd.stderr(Stdio::piped());

    Ok(cmd.spawn()?)
}

pub fn probe_video(path: &str) -> Result<StreamData> {
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
        path,
    ]);
    let output = ensure_command(&mut cmd)?;
    println!("done");

    #[derive(Deserialize)]
    struct ProbeData {
        streams: Vec<StreamDataExt>,
    }

    #[derive(Deserialize)]
    struct StreamDataExt {
        codec_type: String,
        r_frame_rate: String,
        avg_frame_rate: String,
        nb_read_frames: String,
    }

    let mut probe_data = serde_json::from_slice::<ProbeData>(&output.stdout)?;
    if probe_data.streams.len() != 1 {
        bail!("{} video streams in file", probe_data.streams.len());
    }

    let stream = probe_data.streams.pop().unwrap();

    // Sanity checks on stream data
    if stream.codec_type != "video" {
        bail!("codec_type = {}", stream.codec_type);
    }
    if stream.r_frame_rate != stream.avg_frame_rate {
        bail!("video has variable framerate");
    }

    Ok(StreamData {
        framerate: stream.r_frame_rate,
        frames: stream.nb_read_frames.parse()?,
    })
}

#[derive(Debug)]
pub struct StreamData {
    // ffmpeg/ffprobe use strings of fractions for precise framerate, eg "30/1" for 30fps
    pub framerate: String,
    pub frames: u64,
}

// start_frame + count going past the final frame is fine.
pub fn extract_frames(video: &str, output_dir: &str, start_frame: u64, count: u64) -> Result<()> {
    let output_pattern = format!("{output_dir}/frame%08d.png");
    let frame_filter = format!(
        "select='between(n\\,{}\\,{})'",
        start_frame,
        start_frame + count - 1
    );

    let mut cmd = Command::new("ffmpeg");
    cmd.args(BASIC_ARGS);
    cmd.args([
        "-i",
        video,
        "-vf",
        &frame_filter,
        "-fps_mode",
        "passthrough",
        &output_pattern,
    ]);
    ensure_command(&mut cmd)?;
    Ok(())
}
