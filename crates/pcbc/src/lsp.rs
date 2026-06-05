use clap::Args;

#[derive(Args)]
pub struct LspArgs {
    /// Disable network access; use vendored dependencies and cached BOM matches
    #[arg(long = "offline")]
    pub offline: bool,
}

pub fn execute(args: LspArgs) -> anyhow::Result<()> {
    pcb_zen::lsp_with_custom_request_handler(
        false,
        args.offline,
        |_method, _params| Ok(None),
        |_source_path, _schematic| {},
    )
}
