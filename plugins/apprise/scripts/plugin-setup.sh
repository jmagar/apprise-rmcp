#!/usr/bin/env bash
# SessionStart / ConfigChange hook for the Apprise plugin.
set -euo pipefail

binary="${APPRISE_MCP_BIN:-rapprise}"

if ! command -v "${binary}" >/dev/null 2>&1; then
  printf 'apprise plugin setup: rapprise is not installed or not on PATH.\n' >&2
  printf 'Install rapprise separately, then run: rapprise setup\n' >&2
  exit 0
fi

exec "${binary}" setup plugin-hook "$@"
