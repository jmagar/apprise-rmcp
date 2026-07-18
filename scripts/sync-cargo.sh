#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The repository lockfile is canonical; never copy it into plugin data directories.
exec cargo fetch --locked --manifest-path "${repo_root}/Cargo.toml"
