#!/usr/bin/env bash
# Bootstrap the canonical installer from an immutable release tag with bounded
# timeout/redirect handling, SHA256 validation, and provenance verification.
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
command -v gh >/dev/null 2>&1 || { echo "error: GitHub CLI 2.68+ is required for provenance verification" >&2; exit 1; }
gh_version="$(gh --version | head -1)"
if [[ ! "$gh_version" =~ gh\ version\ ([0-9]+)\.([0-9]+)\. ]]; then
  echo "error: unable to determine GitHub CLI version" >&2
  exit 1
fi
if (( BASH_REMATCH[1] < 2 || (BASH_REMATCH[1] == 2 && BASH_REMATCH[2] < 68) )); then
  echo "error: GitHub CLI 2.68+ is required for provenance verification" >&2
  exit 1
fi
temporary_dir="$(mktemp -d)"
trap 'rm -rf "$temporary_dir"' EXIT
base="https://github.com/${repo}/releases/download/${version}/rapprise-installer.sh"
curl_args=(--fail --silent --show-error --location --connect-timeout 10 --max-time 30 --max-redirs 3 \
  --proto '=https' --proto-redir '=https')
curl "${curl_args[@]}" "$base" -o "$temporary_dir/rapprise-installer.sh"
curl "${curl_args[@]}" "$base.sha256" -o "$temporary_dir/rapprise-installer.sh.sha256"
curl "${curl_args[@]}" "$base.sigstore.json" -o "$temporary_dir/rapprise-installer.sh.sigstore.json"
expected="$(awk 'NR == 1 {print $1}' "$temporary_dir/rapprise-installer.sh.sha256")"
actual="$(sha256sum "$temporary_dir/rapprise-installer.sh" | awk '{print $1}')"
[[ "$expected" =~ ^[0-9a-fA-F]{64}$ && "${actual,,}" == "${expected,,}" ]] || {
  echo "error: installer checksum verification failed" >&2
  exit 1
}
gh attestation verify "$temporary_dir/rapprise-installer.sh" --repo "$repo" \
  --bundle "$temporary_dir/rapprise-installer.sh.sigstore.json" \
  --signer-workflow "$repo/.github/workflows/release.yml" \
  --source-ref "refs/tags/$version" --deny-self-hosted-runners >/dev/null || {
  echo "error: installer provenance verification failed" >&2
  exit 1
}
exec bash "$temporary_dir/rapprise-installer.sh" "$@"
