#!/usr/bin/env bash
# Bootstrap the canonical installer from an immutable release tag.
set -euo pipefail

if [[ -x "$(dirname "${BASH_SOURCE[0]}")/scripts/install.sh" ]]; then
  exec "$(dirname "${BASH_SOURCE[0]}")/scripts/install.sh" "$@"
fi

version="${APPRISE_RMCP_VERSION:-}"
repo="${APPRISE_RMCP_REPO:-jmagar/apprise-rmcp}"
if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "error: APPRISE_RMCP_VERSION must be an explicit release tag such as v0.1.3" >&2
  exit 1
fi
temporary="$(mktemp)"
trap 'rm -f "$temporary"' EXIT
curl --fail --silent --show-error --location --connect-timeout 10 --max-time 30 --max-redirs 3 \
  --proto '=https' --proto-redir '=https' \
  "https://raw.githubusercontent.com/${repo}/${version}/scripts/install.sh" -o "$temporary"
exec bash "$temporary" "$@"
