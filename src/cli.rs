use clap::{Parser, ValueEnum};
use eyre::Result;
use std::fs;

#[derive(Parser)]
pub struct CliOptions {
    /// Source video
    pub input: String,

    /// Destination video. [default: x.mp4 -> x_upscaled.mp4]
    /// May be an output directory, in which case the input name is preserved.
    #[arg(name = "OUTPUT", verbatim_doc_comment)]
    maybe_output: Option<String>,

    /// Actual output value, inferred from maybe_output
    #[arg(hide = true, required = false, default_value = "")]
    pub output: String,

    /// Number of frames processed at once.
    #[arg(short = 'w', long = "window", default_value_t = 100)]
    pub window_size: u64,

    /// Scale factor
    #[arg(short = 's', long = "scale", default_value = "2")]
    pub scale: Scale,

    /// Real-ESRGAN upscaling model to use
    #[arg(short = 'm', long = "model", default_value = "realesr-animevideov3")]
    pub model: Model,

    /// Automatically convert vfr to cfr (will store whole video in /tmp!)
    #[arg(short = 'c', long = "convert-vfr")]
    pub convert_vfr: bool,
}

impl CliOptions {
    pub fn parse() -> Result<Self> {
        let mut opts = <CliOptions as Parser>::parse();
        match opts.maybe_output.take() {
            Some(output) => {
                opts.output = if fs::metadata(&output)?.is_dir() {
                    format!("{output}/{}", as_mp4(&opts.input))
                } else {
                    output
                };
            }
            None => opts.output = default_output(&opts.input),
        }
        Ok(opts)
    }
}

#[derive(ValueEnum, Clone, Copy)]
pub enum Scale {
    #[value(name = "1")]
    One,
    #[value(name = "2")]
    Two,
    #[value(name = "3")]
    Three,
    #[value(name = "4")]
    Four,
}

impl Scale {
    pub fn as_str(self) -> &'static str {
        match self {
            Scale::One => "1",
            Scale::Two => "2",
            Scale::Three => "3",
            Scale::Four => "4",
        }
    }
}

#[derive(ValueEnum, Clone, Copy)]
pub enum Model {
    #[value(name = "realesr-animevideov3")]
    Realesranimevideov3,
    #[value(name = "realesrgan-x4plus")]
    Realesrganx4plus,
    #[value(name = "realesrgan-x4plus-anime")]
    Realesrganx4plusanime,
    #[value(name = "realesrnet-x4plus")]
    Realesrnetx4plus,
}

impl Model {
    pub fn as_str(self) -> &'static str {
        match self {
            Model::Realesranimevideov3 => "realesr-animevideov3",
            Model::Realesrganx4plus => "realesrgan-x4plus",
            Model::Realesrganx4plusanime => "realesrgan-x4plus-anime",
            Model::Realesrnetx4plus => "realesrnet-x4plus",
        }
    }
}

fn default_output(input: &str) -> String {
    format!("{}_upscaled.mp4", input.rsplit_once('.').unwrap().0)
}

fn as_mp4(input: &str) -> String {
    format!("{}.mp4", input.rsplit_once('.').unwrap().0)
}
