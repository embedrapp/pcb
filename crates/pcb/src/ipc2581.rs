use clap::{Args, Subcommand};
use std::path::PathBuf;

use pcb_ipc2581_tools::{OutputFormat, UnitFormat, ViewMode, commands, utils};

#[derive(Args)]
pub struct Ipc2581Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show high-level board summary
    Info {
        /// IPC-2581 XML file to inspect
        #[arg(value_hint = clap::ValueHint::FilePath)]
        file: PathBuf,
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
        #[arg(short, long, default_value = "mm")]
        units: UnitFormat,
    },
    /// Export a filtered view of an IPC-2581 file for a specific mode
    View {
        /// Input IPC-2581 XML file
        #[arg(value_hint = clap::ValueHint::FilePath)]
        input: PathBuf,
        #[arg(short, long)]
        mode: ViewMode,
        #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
        output: PathBuf,
    },
    /// Export board summary and stackup to HTML
    Html {
        /// IPC-2581 XML file to export
        #[arg(value_hint = clap::ValueHint::FilePath)]
        file: PathBuf,
        /// Output HTML file path
        #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
        output: Option<PathBuf>,
        /// Unit format for dimensions
        #[arg(short, long, default_value = "mm")]
        units: UnitFormat,
    },
}

pub fn execute(args: Ipc2581Args) -> anyhow::Result<()> {
    utils::color::init_color();

    match args.command {
        Commands::Info {
            file,
            format,
            units,
        } => commands::info::execute(&file, format, units),
        Commands::View {
            input,
            mode,
            output,
        } => commands::view::execute(&input, mode, &output),
        Commands::Html {
            file,
            output,
            units,
        } => commands::html_export::execute(&file, output.as_deref(), units),
    }
}
