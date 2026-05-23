# Local-Only Fork Maintenance Guide

This fork keeps the open local Zener/PCB toolchain and removes Diode-hosted service flows.
Use this guide when changing the fork or rebasing it onto `diodeinc/pcb`.

## Product Boundary

The default `pcb` binary is local-first. It should keep:

- `build`, `layout`, `fmt`, `open`, `info`, `import`, `doc`, `test`, `simulate`, `vendor`, `migrate`, `update`
- local `ipc2581` inspection/editing
- the embedded `stdlib/` materialized into workspaces
- local KiCad/library/dependency resolution needed for builds

The default `pcb` binary should not include:

- `auth`
- top-level `bom`
- `scan`
- `search`
- `route`
- `preview`
- `publish`
- `self update` or automatic update checks
- remote sandbox or `diode://` URI handling
- Diode component library, datasheet, availability, routing, release upload, auth, or MCP service tools

`crates/pcb-diode-api` is intentionally deleted in this fork. If upstream changes need code from it, extract only local, service-independent helpers into another crate instead of re-enabling Diode services.

## Sync Model

- Treat `upstream` (`diodeinc/pcb`) as read-only.
- Keep `main` as a clean mirror of `upstream/main`.
- Keep downstream work on `embedr/release` or another `embedr/*` branch.
- Do not commit personal fork changes directly to `main`.
- Preserve unrelated dirty or untracked files, especially generated release artifacts.

Typical sync:

```bash
git switch main
git fetch upstream
git merge --ff-only upstream/main

git switch embedr/release
git rebase main
```

During conflicts, accept upstream for local runtime/compiler/layout/stdlib fixes, but keep this fork's service removals. If upstream has moved command code from `crates/pcb` into `crates/pcbc`, reapply the same product boundary there. In upstream versions with a `pcb` shim plus `pcbc` toolchain split, this fork should ship the local compiler directly and should not restore CDN/toolchain-install behavior.

## Windows Cache Fix

This branch includes downstream Windows cache work that may not exist upstream:

- `d6551e57` - `Fix Windows workspace cache setup`
- `975bf16f` - `Fix stale Windows cache junction replacement`

When rebasing, preserve the behavior in `crates/pcb-zen/src/cache_index.rs`:

- `.pcb/cache` should point at the user cache.
- On Windows, symlink creation should fall back to a junction when privileges are missing.
- Existing stale symlinks or junctions should be removed safely before replacement.
- Do not replace this with a Unix-only symlink implementation.

If upstream changes `ensure_workspace_cache_symlink`, reconcile carefully and run a focused `pcb-zen` check.

## Dependency And Feature Rules

- `crates/pcb/Cargo.toml` default features should not enable service integrations.
- `pcb-diode-api` should remain deleted and should not be reintroduced as a dependency of `pcb` or `pcb-ipc2581-tools`.
- Do not add a new default feature that performs auth, network service calls, remote uploads, or availability lookups.
- Network access for fetching generic Git/KiCad dependencies is still part of normal package resolution unless the user passes `--offline`.

## Verification

After service-boundary changes, run narrow checks first:

```bash
cargo check -p pcb
cargo run -p pcb -- help
cargo run -p pcb -- build examples/PhaseDriver/PhaseDriver.zen
```

The help output must not list `auth`, `bom`, `scan`, `search`, `route`, `preview`, `publish`, or `self`.

Before a meaningful push or release build, follow the repository-wide checks in `AGENTS.md`.
