#!/usr/bin/env bash
set -euo pipefail

if (( $# == 0 )); then
  echo "Usage: bin/with-downstream-target.sh COMMAND [ARG...]" >&2
  exit 2
fi

root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
common_dir="$(git -C "$root_dir" rev-parse --path-format=absolute --git-common-dir)"
target_dir="$(git -C "$root_dir" config --path --get pcb.releaseTargetDir || true)"
target_dir="${target_dir:-$common_dir/embedr-release-target}"

if [[ "$target_dir" != /* || "$target_dir" == / || "$target_dir" == "$root_dir" || "$target_dir" == "$common_dir" ]]; then
  echo "pcb.releaseTargetDir must name an absolute build-cache directory" >&2
  exit 2
fi
if ! command -v flock >/dev/null; then
  echo "Managed release validation requires flock (util-linux)" >&2
  exit 2
fi
test -f "$root_dir/lib/std/pcb.toml"

# Keep the lock outside the cache so an idle cache can be removed safely.
# Hold it through tests and staging, beyond Cargo's compilation lock.
mkdir -p -- "$(dirname -- "$target_dir")"
exec 9>"$target_dir.lock"
flock --exclusive 9

mkdir -p -- "$target_dir/lib"
if [[ -e "$target_dir/lib/std" && ! -L "$target_dir/lib/std" ]]; then
  echo "Refusing to replace non-symlink $target_dir/lib/std" >&2
  exit 1
fi
ln -sfn -- "$root_dir/lib/std" "$target_dir/lib/std"

export CARGO_TARGET_DIR="$target_dir"
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-0}"
"$@"
