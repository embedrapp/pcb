# Zener VS Code LSP extension

A VSCode LSP extension that talks over stdin/stdout to a binary. This can either
be the pcb binary itself, or any binary that has implemented
`starlark::lsp::server::LspContext` and runs
`starlark::lsp::server::stdio_server()`.

The setting to be aware of is `zener.pcbPath`, which points to the `pcb`
binary used for the language server, formatting, and layout commands. It is
available in the VSCode extension settings UI.

Based on a combination of:

- Tutorial at
  https://code.visualstudio.com/api/language-extensions/language-server-extension-guide
- Code for the tutorial at
  https://github.com/microsoft/vscode-extension-samples/tree/master/lsp-sample
- Syntax files from https://github.com/phgn0/vscode-starlark (which are the
  Microsoft Python ones with minor tweaks)

## Pre-requisites

You need to have npm v7+ installed. Afterwards, run `npm install` in this folder,
in `client`, and in `preview`.

## Debugging

- Follow steps in Pre-requisites section.
- Open VS Code on this folder.
- Press Ctrl+Shift+B to compile the client and server.
- Switch to the Debug viewlet.
- Select `Launch Client` from the drop down.
- Run the launch config.

## Installing

- Follow steps in Pre-requisites section.
- Run `npm install vsce`
- Run `npm exec vsce package`
- In VS Code, go to Extensions, click on the "..." button in the Extensions bar,
  select "Install from VSIX" and then select the `zener-1.0.0.vsix` file that
  was produced.
- Build the pcb binary with `cargo build --bin=pcb` and then do one
  of:
  - Put it on your `$PATH`, e.g.
    `cp $CARGO_TARGET_DIR/debug/pcb ~/.cargo/bin/pcb`.
  - Configure the setting `zener.pcbPath` for this extension to point to the
    pcb binary. e.g. `$CARGO_TARGET_DIR/debug/pcb`.

## Updating

Every few months security advisories will arrive about pinned versions of
packages.

- `npm audit` to see which packages have security updates.
- `npm audit fix` to fix those issues.
- Try `npm audit`, if it still has issues run `npm update`.
- `npm exec vsce package` to confirm everything still works.
