use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete_command::Shell;
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
        Commands::Completions { shell } => {
            shell.generate(&mut Cli::command(), &mut std::io::stdout());
        }
    }

    Ok(())
}
