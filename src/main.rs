use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete_command::Shell;
use ruti::image::pipeline::Job;
use ruti::{color, image};
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(about,long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Color related utilities
    Color {
        #[command(subcommand)]
        cmd: ColorCmds,
    },

    /// Image related utilities
    Image {
        /// Paths to input images
        #[arg(required = true)]
        images: Vec<String>,

        /// Output image quality.
        #[arg(short, long)]
        quality: Option<u8>,

        /// Background color to use when converting images with transparency to a format
        /// that does not support alpha. Supports hex, RGB, RGBA, Oklab, and Oklch
        /// color formats.
        #[arg(short, long)]
        background: Option<String>,

        /// Target dimensions in WIDTHxHEIGHT format (e.g., 1920x1080).
        #[arg(short, long)]
        size: Option<String>,

        /// Resize strategy to use when resizing images. Requires `--size`.
        #[arg(
            value_enum,
            long,
            short = 'r',
            default_value = "resize",
            requires = "size"
        )]
        resize: image::resize::ResizeType,

        /// Directory in which to save the output images.
        #[arg(short, long)]
        out_dir: Option<String>,

        /// Output image format
        #[arg(short, long, value_enum, required = true)]
        format: image::codecs::OutputFormat,
    },

    /// Generate shell completions
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum ColorCmds {
    /// Convert one or more colors to a target color space.
    Convert {
        /// Colors to convert. Note: Remember to enclose the color values with double quote e.g.
        /// "#fff"
        colors: Vec<String>,

        /// Output color space
        #[arg(short, long, value_enum)]
        output: color::Color,

        /// Only output converted color values
        #[arg(long, default_value_t = false)]
        output_only: bool,
    },

    /// Replace colors from a file with converted color values
    Replace {
        /// Pattern to match. [possible values: hex, rgb, rgba, oklab, oklch, exact color string
        /// (regexes are not allowed)]
        #[arg(short, long)]
        pattern: String,

        /// Path to file
        #[arg(short, long)]
        file: PathBuf,

        /// Output format for matched colors
        #[arg(short, long, value_enum)]
        output: color::Color,

        #[arg(short, long, default_value_t = false)]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Color { cmd } => match cmd {
            ColorCmds::Convert {
                colors,
                output,
                output_only,
            } => {
                let conversions = color::convert_colors(&colors, &output);

                let stdout = io::stdout();
                let stderr = io::stderr();

                let mut w_writer = io::BufWriter::new(stdout.lock());
                let mut l_writer = io::BufWriter::new(stderr.lock());

                color::log_conversion_results(
                    &conversions,
                    output_only,
                    &mut w_writer,
                    &mut l_writer,
                );
            }
            ColorCmds::Replace {
                pattern,
                file,
                output,
                dry_run,
            } => {
                color::replace_in_file(file, &pattern, output, dry_run)?;
            }
        },

        Commands::Image {
            images,
            quality,
            background,
            size,
            resize,
            out_dir,
            format,
        } => {
            let jobs: Vec<Job> = images
                .iter()
                .map(|i| {
                    if format == image::codecs::OutputFormat::Jpeg {
                        let color = color::convert(
                            &background.clone().unwrap_or(String::from("#fff")),
                            &color::Color::Rgba,
                        )
                        .unwrap_or(String::from("rgba(255,255,255)"));

                        let rgba = color::Rgba::from_str(&color).unwrap_or(color::Rgba {
                            r: 255u8,
                            g: 255u8,
                            b: 255u8,
                            alpha: 1.0,
                        });

                        return Job::new(
                            resize.clone(),
                            size.clone(),
                            format.clone(),
                            PathBuf::from(i),
                            quality.unwrap_or(75u8),
                            Some([rgba.r, rgba.g, rgba.b]),
                        );
                    }

                    Job::new(
                        resize.clone(),
                        size.clone(),
                        format.clone(),
                        PathBuf::from(i),
                        quality.unwrap_or(75u8),
                        None,
                    )
                })
                .collect();

            let outdir = PathBuf::from(out_dir.unwrap_or(String::from(".")));

            image::pipeline::process(jobs, &outdir);
        }

        Commands::Completions { shell } => {
            shell.generate(&mut Cli::command(), &mut std::io::stdout());
        }
    }

    Ok(())
}
