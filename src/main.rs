// #![allow(dead_code, unused_imports, unreachable_code)]
mod cli;
mod ffmpeg;
mod png_stream;
mod realesrgan;
mod util;

use crate::{
    cli::CliOptions,
    ffmpeg::VFRError,
    util::{clear_line, command, file_exists, pretty_time, print_flush, progress_bar},
};
use eyre::{bail, ensure, Context, Result};
use std::{
    cmp::min,
    fs::{self, File},
    io::{self, BufReader},
    process,
    ptr::null_mut,
    time::Instant,
};

fn main() -> Result<()> {
    let options = CliOptions::parse();
    let input = &options.input;
    let output = &options.output;

    // Helpful for analyzing output from shell loops
    println!("input = {input}");
    println!("output = {output}");

    // Check input and output files
    ensure!(file_exists(input)?, "{input} doesn't exist");
    ensure!(!file_exists(output)?, "{output} already exists");

    // Automatically download Real-ESRGAN
    realesrgan::check_and_download().context("downloading Real-ESRGAN")?;

    let CliOptions {
        input,
        output,
        window_size,
        scale,
        model,
        convert_vfr: handle_vfr,
        ..
    } = options;

    upscale_video(
        &input,
        &output,
        window_size,
        scale.as_str(),
        model.as_str(),
        handle_vfr,
    )
    .context("Reencoding failed!")
    .map_err(|e| {
        // TODO: Make Command wrapper that waits for child process on drop,
        // so this call is unnecessary
        unsafe { libc::wait(null_mut()) };
        _ = fs::remove_file(output);
        e
    })
}

fn upscale_video(
    input: &str,
    output: &str,
    window_size: u64,
    scale: &str,
    model: &str,
    convert_vfr: bool,
) -> Result<()> {
    // Interrogate input video for frame info
    let stream_data_or_vfr = ffmpeg::probe_video(input)?;
    let stream_data = match stream_data_or_vfr {
        Ok(sd) => sd,
        Err(VFRError) if !convert_vfr => bail!("variable framerate input (try -c)"),
        Err(VFRError) => {
            print_flush!("Converting vfr video to cfr...");
            let (_conversion_temp_dir, converted) = ffmpeg::convert_vfr_to_cfr(input)?;
            println!("done");
            let new_input = converted.as_os_str().to_str().unwrap();
            return upscale_video(new_input, output, window_size, scale, model, false);
        }
    };
    let ffmpeg::StreamData { frames, framerate } = stream_data;

    println!("Upscaling video...");
    let start_time = Instant::now();

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

    let mut remaining_secs: Option<u64> = None;
    let n_windows = frames / window_size + 1;
    for window_i in 0..n_windows {
        let window_start_time = Instant::now();
        let first = window_i * window_size;
        let last = min(frames, first + window_size);
        let window_frames = last - first;

        clear_line();
        print_flush!("Window {window_i:02}/{n_windows:02} Frame {first:03}/{frames:03}");
        print_flush!(" {}", progress_bar(30, first as usize, frames as usize));
        if let Some(rs) = remaining_secs {
            print_flush!(" (est. {})", pretty_time(rs));
        }

        // Write frames from decoder into lores frames dir
        for frame_i in 0..window_frames {
            let f = File::create(format!("{lores_frames_dir}/frame{frame_i:04}.png"))?;
            png_stream.write_next(f)?;
        }

        // Upscale from lores dir to hires dir
        realesrgan::upscale_images_in_dir(lores_frames_dir, hires_frames_dir, scale, model)?;

        // Write frames from hires frames dir into encoder, and delete them
        for fi in 0..window_frames {
            let lores_frame_path = format!("{lores_frames_dir}/frame{fi:04}.png");
            let hires_frame_path = format!("{hires_frames_dir}/frame{fi:04}.png");
            let mut hires_frame = BufReader::new(File::open(&hires_frame_path)?);
            io::copy(&mut hires_frame, &mut encoder_input)?;
            fs::remove_file(lores_frame_path)?;
            fs::remove_file(hires_frame_path)?;
        }

        // Estimate remaining time based on how long this window took
        let window_secs_elapsed = Instant::now().duration_since(window_start_time).as_secs();
        remaining_secs = Some(window_secs_elapsed * (frames - last) / window_frames);
    }

    let secs_elapsed = Instant::now().duration_since(start_time).as_secs();

    // Clear progress line & print final stats
    clear_line();
    println!("Upscaled {frames} frames in {}", pretty_time(secs_elapsed));

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
    fs::read_to_string(format!("/proc/self/task/{}/children", process::id()))
        .expect("getting pids of child processes")
        .split_whitespace()
        .for_each(|pid| _ = command!("kill", pid));
}
