#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "${fixture}"' EXIT
mkdir -p "${fixture}/release/v9.9.9" "${fixture}/payload" "${fixture}/bin"
printf '#!/bin/sh\necho verified\n' > "${fixture}/payload/rapprise"
chmod 755 "${fixture}/payload/rapprise"
tar -C "${fixture}/payload" -czf "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz" rapprise
sha256sum "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz" > "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz.sha256"

APPRISE_RMCP_VERSION=v9.9.9 \
APPRISE_RMCP_RELEASE_BASE_URL="file://${fixture}/release" \
APPRISE_RMCP_CURL_PROTOCOLS='=file' \
INSTALL_DIR="${fixture}/bin" \
  "${repo_root}/scripts/install.sh"
[[ "$("${fixture}/bin/rapprise")" == verified ]]

printf old > "${fixture}/bin/rapprise"
printf '0%.0s' {1..64} > "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz.sha256"
if APPRISE_RMCP_VERSION=v9.9.9 APPRISE_RMCP_RELEASE_BASE_URL="file://${fixture}/release" \
  APPRISE_RMCP_CURL_PROTOCOLS='=file' INSTALL_DIR="${fixture}/bin" "${repo_root}/scripts/install.sh"; then
  echo "error: tampered archive unexpectedly installed" >&2
  exit 1
fi
[[ "$(cat "${fixture}/bin/rapprise")" == old ]]
echo "installer contract tests passed"
