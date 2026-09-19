use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io;
use std::path::PathBuf;
mod color;

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
}

#[derive(Subcommand)]
enum ColorCmds {
    /// Convert one or more colors to a target color space.
    Convert {
        /// Colors to convert. Note: Remember to enclose the color values with double quote e.g.
        /// "#fff"
        colors: Vec<String>,

        /// Output color space (hex, rgb, rgba, oklab, oklch)
        #[arg(short, long, value_enum)]
        output: color::Color,

        /// Only output converted color values
        #[arg(long, default_value_t = false)]
        output_only: bool,
    },

    /// Replace colors from a file with converted color values
    Replace {
        /// Pattern to match
        #[arg(short, long, value_enum)]
        pattern: color::Color,

        /// Path to file
        #[arg(short, long)]
        file: PathBuf,
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
            ColorCmds::Replace { pattern, file } => {
                println!("pattern: {:?} file: {}", pattern, file.display())
            }
        },
    }

    Ok(())
}
