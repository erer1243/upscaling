mod ffmpeg;
mod realesrgan;
mod util;

use eyre::{bail, Context, Result};
use std::{
    fs,
    io::{self, ErrorKind},
    process::{exit, Command},
};
use util::print_flush;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("usage: {} INPUT OUTPUT", args[0]);
        exit(1);
    }

    reencode_video(
        &args[1],
        &args[2],
        Options {
            frame_chunk_size: 10,
        },
    )
    .context("Reencoding failed!")
}

struct Options {
    // Number of frames stored in tempdir at one time
    frame_chunk_size: u64,
}

fn reencode_video(input: &str, output: &str, opts: Options) -> Result<()> {
    realesrgan::check_and_download().context("downloading Real-ESRGAN")?;

    match fs::metadata(output) {
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Ok(_) => bail!("output file already exists"),
        e => _ = e?,
    }

    let input_stream = ffmpeg::probe_video(input)?;

    let temp_dir = util::TempDir::new()?;
    let lores_frames_dir = temp_dir.path().join("in");
    let hires_frames_dir = temp_dir.path().join("out");
    fs::create_dir(&lores_frames_dir)?;
    fs::create_dir(&hires_frames_dir)?;

    let lores_frames_dir = lores_frames_dir.to_str().unwrap();
    let hires_frames_dir = hires_frames_dir.to_str().unwrap();

    let mut encoder_proc = ffmpeg::launch_encoder(&input_stream.framerate, input, output)?;
    let mut encoder_input = encoder_proc.stdin.take().unwrap();

    ctrlc::set_handler(ctrlc_handler)?;

    let frame_chunk_size = opts.frame_chunk_size;
    let frames = input_stream.frames;
    let n_chunks = frames / frame_chunk_size + 1;

    for chunk_i in 0..n_chunks {
        let start_frame = chunk_i * frame_chunk_size;
        let end_frame = (start_frame + frame_chunk_size - 1).min(frames);
        print_flush!("Processing frames {start_frame:03}-{end_frame:03} out of {frames:03}... ");

        ffmpeg::extract_frames(input, lores_frames_dir, start_frame, frame_chunk_size)?;
        realesrgan::upscale_images_in_dir(lores_frames_dir, hires_frames_dir, "2")?;

        let frames_processed = end_frame - start_frame;
        for frame_i in 0..frames_processed {
            let frame_i = frame_i + 1; // ffmpeg adds 1 to frame number
            let lores_frame_path = format!("{lores_frames_dir}/frame{frame_i:08}.png");
            let hires_frame_path = format!("{hires_frames_dir}/frame{frame_i:08}.png");
            let upscaled_frame = fs::read(&hires_frame_path)?;
            io::copy(&mut upscaled_frame.as_slice(), &mut encoder_input)?;
            fs::remove_file(lores_frame_path)?;
            fs::remove_file(hires_frame_path)?;
        }

        println!("done");
    }

    drop(encoder_input);
    encoder_proc.wait()?;

    Ok(())
}

fn ctrlc_handler() {
    println!("Interrupted");

    // Kill all child procs. This will allow normal error handling to take over, and run drop code.
    let our_pid = std::process::id();
    let children_str = fs::read_to_string(&format!("/proc/self/task/{our_pid}/children"))
        .expect("getting pids of child processes");
    for pid in children_str.trim().split_whitespace() {
        _ = Command::new("kill").arg(pid).status();
    }
}
