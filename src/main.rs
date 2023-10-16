// #![allow(dead_code, unused_imports, unreachable_code)]
mod ffmpeg;
mod png_stream;
mod realesrgan;
mod util;

use eyre::{ensure, Context, Result};
use std::{
    cmp::min,
    fs::{self, File},
    io::{self, BufReader},
    process::{exit, Command},
};
use util::print_flush;

use crate::util::file_exists;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("usage: {} INPUT OUTPUT", args[0]);
        exit(1);
    }

    let input = &args[1];
    let output = &args[2];
    let options = Options {
        frame_window_size: 10,
    };

    let res = reencode_video(input, output, options).context("Reencoding failed!");
    if res.is_err() {
        _ = fs::remove_file(output);
    }
    res
}

struct Options {
    // Number of frames stored in tempdir at one time
    frame_window_size: u64,
}

fn reencode_video(input: &str, output: &str, opts: Options) -> Result<()> {
    // Automatically download Real-ESRGAN
    realesrgan::check_and_download().context("downloading Real-ESRGAN")?;

    // Check input and output files
    ensure!(file_exists(input)?, "input file doesn't exist");
    ensure!(!file_exists(output)?, "output file already exists");

    // Interrogate input video for frame info
    let ffmpeg::StreamData { frames, framerate } = ffmpeg::probe_video(input)?;

    // Setup tempdir used as work space for realesrgan
    let temp_dir = util::TempDir::new()?;
    let lores_frames_dir = temp_dir.path().join("in");
    let hires_frames_dir = temp_dir.path().join("out");
    fs::create_dir(&lores_frames_dir)?;
    fs::create_dir(&hires_frames_dir)?;
    let lores_frames_dir = lores_frames_dir.to_str().unwrap();
    let hires_frames_dir = hires_frames_dir.to_str().unwrap();

    // Launch ffmpeg encoder
    let mut encoder_proc = ffmpeg::launch_encoder(&framerate, input, output)?;
    let mut encoder_input = encoder_proc.stdin.take().unwrap();

    // Launch ffmpeg decoder
    let mut decoder_proc = ffmpeg::launch_decoder(input)?;
    let decoder_output = decoder_proc.stdout.take().unwrap();
    let mut png_stream = png_stream::PngStreamSplitter::new(decoder_output);

    ctrlc::set_handler(ctrlc_handler)?;

    let window_size = opts.frame_window_size;
    let n_windows = frames / window_size + 1;

    for window_i in 0..n_windows {
        let first = window_i * window_size;
        let last = min(frames, first + window_size);
        let n_frames = last - first;
        print_flush!("Processing frames {first:03}/{frames:03}... ");

        // Write frames from decoder into lores frames dir
        for frame_i in 0..n_frames {
            let f = File::create(format!("{lores_frames_dir}/frame{frame_i:04}.png"))?;
            png_stream.write_next(f)?;
        }

        // Upscale from lores dir to hires dir
        realesrgan::upscale_images_in_dir(lores_frames_dir, hires_frames_dir, "2")?;

        // Write frames from hires frames dir into encoder, and delete them
        for frame_i in 0..n_frames {
            let lores_frame_path = format!("{lores_frames_dir}/frame{frame_i:04}.png");
            let hires_frame_path = format!("{hires_frames_dir}/frame{frame_i:04}.png");
            let mut hires_frame = BufReader::new(File::open(&hires_frame_path)?);
            io::copy(&mut hires_frame, &mut encoder_input)?;
            fs::remove_file(lores_frame_path)?;
            fs::remove_file(hires_frame_path)?;
        }

        println!("done");
    }

    // End ffmpeg processes
    drop(png_stream);
    drop(encoder_input);
    decoder_proc.wait()?;
    encoder_proc.wait()?;

    Ok(())
}

fn ctrlc_handler() {
    println!("Interrupted");

    // Kill all child procs. This will allow normal error propagation to take over, and run drop code.
    let our_pid = std::process::id();
    let children_str = fs::read_to_string(&format!("/proc/self/task/{our_pid}/children"))
        .expect("getting pids of child processes");
    for pid in children_str.trim().split_whitespace() {
        _ = Command::new("kill").arg(pid).status();
    }
}
