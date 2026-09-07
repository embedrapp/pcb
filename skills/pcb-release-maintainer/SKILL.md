---
name: pcb-release-maintainer
description: Review diodeinc/pcb upstream releases, safely sync Embedr's local-first release branch, and publish downstream pcb binaries through the embedrapp/pcb Downstream Release GitHub Actions workflow. Use for recurring upstream checks and authorized Embedr PCB releases; do not use for upstream publishing or Diode service releases.
---

# PCB Release Maintainer

Maintain and release the `embedrapp/pcb` fork without restoring Diode-hosted service flows.

## Repository and release boundary

- Work only in the managed `embedrapp/pcb` checkout identified by the scheduled prompt. Validate its `origin` before making changes.
- Treat `https://github.com/diodeinc/pcb.git` as read-only upstream. Never push to it.
- Keep `main` as an upstream mirror and put downstream changes on `embedr/release`.
- Preserve `FORK.md`, the local-first CLI boundary, Windows cache behavior, packaged `lib/std`, and existing downstream fixes.
- Do not restore auth, publish, preview, route, search, scan, top-level BOM, self-update, remote sandbox, `diode://`, Diode API/S3 releases, or upstream-only secrets and runners.
- The only publishing workflow for this task is `.github/workflows/downstream-release.yml` in `embedrapp/pcb`, triggered by an `embedr-v*` tag.

## Decide whether to release

Fetch `origin`, upstream, and tags. Resolve the newest stable upstream tag matching exactly `vMAJOR.MINOR.PATCH` that is reachable from `upstream/main`. Compare it with the upstream base recorded by the newest downstream release tag `embedr-vMAJOR.MINOR.PATCH.N`.

- If no newer stable upstream tag exists, leave the repository unchanged and report a no-op with the compared tags.
- Ignore prerelease upstream tags unless the scheduled prompt explicitly asks for them.
- If a release for the newest upstream version already exists, do not create another merely because the schedule ran again.
- If GitHub already has an in-progress Downstream Release for the intended tag or commit, monitor that run instead of creating a duplicate.

## Prepare the downstream release

Use a disposable worktree or otherwise isolate the scheduled run from the shared managed checkout. Preserve unrelated changes and never clean or reset a user's worktree.

1. Start from the current `origin/embedr/release` branch and rebase it onto the chosen stable upstream tag. A major conflict or upstream structural change is a reason to stop and report, not to guess.
2. Reconcile conflicts by keeping upstream compiler, runtime, layout, stdlib, and local tooling improvements while preserving every constraint in `FORK.md`.
3. Set the workspace version to `<upstream-version>-embedr.1` for the first release based on that upstream tag, or increment the downstream suffix only when an explicit follow-up release is justified. Update `Cargo.lock` consistently.
4. Prepare GitHub Release notes from the actual diff between the previous published `embedr-v*` tag and the new `embedr-v*` tag.
5. Add one succinct `CHANGELOG.md` entry under `Unreleased` describing the upstream sync when the repository convention requires it.

## Write product release notes

Release notes are a user-facing changelog for the Embedr PCB builds. Compare the previous
published downstream tag directly with the current downstream tag, and describe only behavior
that users gain, lose, or can observe in that fork-to-fork range.

- Lead with `What's changed since pcb <previous-version>` and group concise bullets by product area.
- Include new commands, capabilities, supported formats, output changes, bug fixes, and meaningful
  performance or correctness improvements that are present and usable in the downstream build.
- Inspect the downstream tag diff. Do not copy changes merely because they appear in another
  project's release notes; exclude anything removed, disabled, or unreachable in this fork.
- Do not mention upstream syncing, rebases, merge conflicts, fork policy, excluded services,
  workflows, CI, validation steps, packaging mechanics, checksums, artifact inventories,
  operational instructions, or other release-process metadata.
- Do not use the full accumulated changelog or compare against an upstream tag. Each release body
  covers exactly the previous published downstream release through the current downstream release.
- The workflow-created release body is only a placeholder. After publication, replace it with the
  curated product notes before reporting the release complete.

## Validate and publish

Run the narrow checks first:

```bash
cargo fmt --check
cargo check -p pcbc
cargo run -p pcbc -- help
```

The help output must not expose `auth`, `bom`, `scan`, `search`, `route`, `preview`, `publish`, `self`, or `toolchain`. Then run the downstream packaged-runtime smoke build used by `.github/workflows/downstream-release.yml`. Run broader tests only when the upstream diff or a conflict resolution makes them relevant. Never accept snapshots automatically.

If validation passes:

1. Commit the sync on `embedr/release` with the repository's existing message style.
2. Re-fetch `origin` and use `--force-with-lease` only when the rebased downstream branch requires it.
3. Create one annotated or lightweight tag named `embedr-v<upstream-version>.1`, after confirming it does not exist locally or on GitHub.
4. Push only `embedr/release` and that tag to `origin`. The tag starts the `Downstream Release` GitHub Actions workflow.
5. Monitor the matching workflow to completion. Confirm the GitHub Release exists and contains Linux x64, macOS universal, Windows x64, per-artifact checksum files, and `SHA256SUMS.txt`.
6. Replace the placeholder GitHub Release body with the curated fork-to-fork product notes described above before reporting success.

Do not manually upload binaries, invoke upstream release workflows, publish to Diode S3, or claim success while GitHub Actions is queued or running. On failure, preserve the run URL and concise error evidence; do not retag or retry more than once without a new diagnosis.

## Report

Leave a concise scheduled-run result containing:

- upstream tag reviewed and previous downstream release;
- no-op, blocked, or released outcome;
- downstream commit and tag when created;
- checks performed;
- GitHub Actions run and GitHub Release URLs; and
- any conflicts, failures, or follow-up needed.
