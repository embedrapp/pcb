use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use pcb_ipc2581_tools::{LayoutTarget, commands};

use crate::layout::LayoutArgs;

#[derive(Args, Debug)]
#[command(about = "Run DFM checks for a .zen board")]
pub struct DfmArgs {
    /// Path to the board .zen file
    #[arg(value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub file: PathBuf,

    /// Built-in PDK name or fabrication PDK TOML path (built-ins: standard)
    #[arg(long)]
    pub pdk: PathBuf,

    /// Output self-contained JSON report path. Omit to write to stdout.
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Disable network access (offline mode) - only use vendored dependencies
    #[arg(long = "offline")]
    pub offline: bool,
}

pub fn execute(args: DfmArgs) -> Result<()> {
    let options = commands::dfm::CheckOptions {
        pdk: args.pdk.clone(),
        waivers: None,
        output: args.output.clone(),
        layout_target: LayoutTarget::Board,
    };
    commands::dfm::validate_output(&args.file, &options)?;
    let (_temporary_dir, ipc_path) = match export_layout(&args) {
        Ok(exported) => exported,
        Err(error) => {
            commands::dfm::write_error_report(&args.file, &options, &error)
                .with_context(|| format!("DFM check was incomplete: {error:#}"))?;
            return Err(error);
        }
    };

    commands::dfm::execute_check(&ipc_path, &options)
}

fn export_layout(args: &DfmArgs) -> Result<(tempfile::TempDir, PathBuf)> {
    let layout_args = LayoutArgs {
        file: args.file.clone(),
        no_open: true,
        offline: args.offline,
        ..Default::default()
    };
    let design = crate::layout::prepare_design(&layout_args)?;
    let layout = crate::layout::apply_prepared(&layout_args, design)?;
    let pcb_file = layout
        .pcb_file_abs
        .as_deref()
        .with_context(|| format!("{} does not declare a layout", args.file.display()))?;

    let temporary_dir = tempfile::tempdir().context("failed to create temporary DFM directory")?;
    let ipc_path = temporary_dir.path().join("ipc2581.xml");
    export_ipc2581(pcb_file, &ipc_path)?;

    Ok((temporary_dir, ipc_path))
}

fn export_ipc2581(kicad_pcb_path: &Path, ipc2581_path: &Path) -> Result<()> {
    pcb_kicad::KiCadCliBuilder::new()
        .command("pcb")
        .subcommand("export")
        .subcommand("ipc2581")
        .arg("--output")
        .arg(ipc2581_path.to_string_lossy())
        .arg("--bom-col-int-id")
        .arg("Path")
        .arg("--bom-col-mfg-pn")
        .arg("Mpn")
        .arg("--bom-col-mfg")
        .arg("Manufacturer")
        .arg(kicad_pcb_path.to_string_lossy())
        .run()
        .context("Failed to generate IPC-2581 file")
}
