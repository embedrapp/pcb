# Local-Only Fork Maintenance Guide

This fork keeps the open local Zener/PCB toolchain and removes Diode-hosted service flows.
Use this guide when changing the fork or rebasing it onto `diodeinc/pcb`.

## Product Boundary

The default `pcb` binary is local-first. It should keep:

- `build`, `layout`, `fmt`, `open`, `info`, `import`, `doc`, `test`, `simulate`, `vendor`, `migrate`, `update`
- local `ipc2581` inspection/editing and BOM exports from local files, `gltf` export, built-in `rectify`, and `new component <DIR>` imports from local files
- the embedded `lib/std` materialized into workspaces
- local KiCad/library/dependency resolution needed for builds

The default `pcb` binary should not include:

- `auth`
- top-level `bom`
- `scan`
- `search`
- `route`
- `preview`
- `publish`
- hosted `quote`
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

During conflicts, accept upstream for local runtime/compiler/layout/stdlib fixes, but keep this fork's service removals. In upstream versions with a `pcb` shim plus `pcbc` toolchain split, this fork ships the local compiler directly as the `pcb` release artifact and does not restore CDN/toolchain-install behavior.

## Windows Cache Fix

This branch includes downstream Windows cache work that may not exist upstream:

- `.pcb/cache` should point at the user cache.
- On Windows, symlink creation should fall back to a junction when privileges are missing.
- Existing stale symlinks or junctions should be removed safely before replacement.
- Do not replace this with a Unix-only symlink implementation.

If upstream changes `ensure_workspace_cache_symlink`, reconcile carefully and run a focused `pcb-zen` check.

## Dependency And Feature Rules

- `pcbc` default features should not enable service integrations.
- `pcb-diode-api` should remain deleted and should not be reintroduced as a dependency of `pcbc` or `pcb-ipc2581-tools`.
- Do not add a new default feature that performs auth, network service calls, remote uploads, or availability lookups.
- Network access for fetching generic Git/KiCad dependencies is still part of normal package resolution unless the user passes `--offline`.

## Managed Release Build Cache

Scheduled Linux release runs must use one stable target cache for this project,
including when building in disposable worktrees. Configure it once in the managed
checkout:

```bash
git config --local pcb.releaseTargetDir /srv/aisevak/.cache/pcb-release-target
```

Run the complete validation and runtime-staging script through:

```bash
bin/with-downstream-target.sh bash <validation-script>
```

The helper exports `CARGO_TARGET_DIR` from that local Git setting, falling back
to `embedr-release-target` in the common Git directory on other hosts. Do not
override it with a fresh per-task target. The dedicated managed Cargo home also
uses the configured shared target as its default.

The helper holds an exclusive lock through compilation, tests, and staging, and
links the current worktree's `lib/std` beside the build artifacts for runtime
discovery. Keep all those steps within one invocation. Concurrent release builds
for this project wait for the lock; other projects and GitHub Actions runners
remain independent. Dependencies are reused between runs; changed workspace
crates can still rebuild.

Stage `pcbc` as `pcb` beside a copy of `lib/std` before removing the disposable
worktree. Preserve the shared cache for later runs. Remove a cache only while its
lock is held and its build, test, DFM, and KiCad processes have exited.

## Verification

After service-boundary changes, run narrow checks first:

```bash
cargo check -p pcbc
cargo run -p pcbc -- help
cargo run -p pcbc -- build examples/PhaseDriver/PhaseDriver.zen
```

The help output must not list `auth`, `bom`, `scan`, `search`, `route`, `preview`, `publish`, `self`, or `toolchain`.
