#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "${fixture}"' EXIT
mkdir -p "${fixture}/release/v9.9.9" "${fixture}/payload" "${fixture}/bin"
mkdir -p "${fixture}/mock-bin"
printf '#!/bin/sh\necho verified\n' > "${fixture}/payload/rapprise"
chmod 755 "${fixture}/payload/rapprise"
tar -C "${fixture}/payload" -czf "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz" rapprise
sha256sum "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz" > "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz.sha256"
printf '{"verificationMaterial":{}}\n' > "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz.sigstore.json"
cat > "${fixture}/mock-bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then echo 'gh version 2.68.0'; exit 0; fi
printf '%s\n' "$*" >> "$GH_LOG"
[[ "${GH_SHOULD_FAIL:-0}" != 1 ]]
EOF
chmod +x "${fixture}/mock-bin/gh"
export PATH="${fixture}/mock-bin:${PATH}"
export GH_LOG="${fixture}/gh.log"

APPRISE_RMCP_VERSION=v9.9.9 \
APPRISE_RMCP_RELEASE_BASE_URL="file://${fixture}/release" \
APPRISE_RMCP_CURL_PROTOCOLS='=file' \
INSTALL_DIR="${fixture}/bin" \
  "${repo_root}/scripts/install.sh"
[[ "$("${fixture}/bin/rapprise")" == verified ]]
grep -F "attestation verify" "$GH_LOG" >/dev/null
grep -F -- "--source-ref refs/tags/v9.9.9" "$GH_LOG" >/dev/null

printf old > "${fixture}/bin/rapprise"
printf '0%.0s' {1..64} > "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz.sha256"
if APPRISE_RMCP_VERSION=v9.9.9 APPRISE_RMCP_RELEASE_BASE_URL="file://${fixture}/release" \
  APPRISE_RMCP_CURL_PROTOCOLS='=file' INSTALL_DIR="${fixture}/bin" "${repo_root}/scripts/install.sh"; then
  echo "error: tampered archive unexpectedly installed" >&2
  exit 1
fi
[[ "$(cat "${fixture}/bin/rapprise")" == old ]]

sha256sum "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz" > "${fixture}/release/v9.9.9/rapprise-x86_64.tar.gz.sha256"
if GH_SHOULD_FAIL=1 APPRISE_RMCP_VERSION=v9.9.9 APPRISE_RMCP_RELEASE_BASE_URL="file://${fixture}/release" \
  APPRISE_RMCP_CURL_PROTOCOLS='=file' INSTALL_DIR="${fixture}/bin" "${repo_root}/scripts/install.sh"; then
  echo "error: archive with unverified provenance unexpectedly installed" >&2
  exit 1
fi
[[ "$(cat "${fixture}/bin/rapprise")" == old ]]
echo "installer contract tests passed"
