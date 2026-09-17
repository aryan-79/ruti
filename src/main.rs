use anyhow::Result;
use clap::{Parser, Subcommand};
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
            ColorCmds::Convert { colors, output } => {
                println!("colors: {:?} output: {:?}", colors, output)
            }
            ColorCmds::Replace { pattern, file } => {
                println!("pattern: {:?} file: {}", pattern, file.display())
            }
        },
    }

    Ok(())
}
