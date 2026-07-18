#!/usr/bin/env bash
set -euo pipefail

REPO="${APPRISE_RMCP_REPO:-jmagar/apprise-rmcp}"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${APPRISE_RMCP_VERSION:-}"
RELEASE_BASE_URL="${APPRISE_RMCP_RELEASE_BASE_URL:-}"
BINARY_NAME="rapprise"
CONNECT_TIMEOUT="${APPRISE_RMCP_CONNECT_TIMEOUT:-10}"
TOTAL_TIMEOUT="${APPRISE_RMCP_TOTAL_TIMEOUT:-120}"
MAX_REDIRECTS="${APPRISE_RMCP_MAX_REDIRECTS:-5}"
CURL_PROTOCOLS="${APPRISE_RMCP_CURL_PROTOCOLS:-=https}"

usage() {
  cat <<'USAGE'
Install a checksum-verified rapprise release archive.

Environment:
  APPRISE_RMCP_VERSION     Required immutable release tag, for example v0.1.3
  INSTALL_DIR              Destination directory (default: ~/.local/bin)
  APPRISE_RMCP_REPO        GitHub repo owner/name (default: jmagar/apprise-rmcp)
  APPRISE_RMCP_RELEASE_BASE_URL  Test/mirror base URL
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "${VERSION}" || ! "${VERSION}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "error: APPRISE_RMCP_VERSION must be an explicit release tag such as v0.1.3" >&2
  exit 1
fi

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "error: $1 is required" >&2; exit 1; }
}

target_asset() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "${os}:${arch}" in
    linux:x86_64|linux:amd64) printf '%s-x86_64.tar.gz' "${BINARY_NAME}" ;;
    mingw*:x86_64|msys*:x86_64|cygwin*:x86_64) printf '%s-windows-x86_64.tar.gz' "${BINARY_NAME}" ;;
    *) echo "error: unsupported platform ${os}/${arch}; supported: linux/x86_64, windows/x86_64" >&2; exit 1 ;;
  esac
}

need curl
need install
need mktemp
need tar

if command -v sha256sum >/dev/null 2>&1; then
  hash_file() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  hash_file() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  echo "error: sha256sum or shasum is required" >&2
  exit 1
fi

asset="$(target_asset)"
tmpdir="$(mktemp -d)"
install_tmp=""
cleanup() {
  rm -rf "${tmpdir}"
  [[ -z "${install_tmp}" ]] || rm -f "${install_tmp}"
}
trap cleanup EXIT

if [[ -n "${RELEASE_BASE_URL}" ]]; then
  url="${RELEASE_BASE_URL%/}/${VERSION}/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

curl_args=(--fail --silent --show-error --location --connect-timeout "${CONNECT_TIMEOUT}" --max-time "${TOTAL_TIMEOUT}" --max-redirs "${MAX_REDIRECTS}" --proto "${CURL_PROTOCOLS}" --proto-redir "${CURL_PROTOCOLS}")
echo "Downloading ${url}" >&2
curl "${curl_args[@]}" "${url}" -o "${tmpdir}/${asset}"
curl "${curl_args[@]}" "${url}.sha256" -o "${tmpdir}/${asset}.sha256"

expected="$(awk 'NR == 1 {print $1}' "${tmpdir}/${asset}.sha256")"
if [[ ! "${expected}" =~ ^[0-9a-fA-F]{64}$ ]]; then
  echo "error: malformed SHA256 file for ${asset}" >&2
  exit 1
fi
actual="$(hash_file "${tmpdir}/${asset}")"
if [[ "${actual,,}" != "${expected,,}" ]]; then
  echo "error: SHA256 mismatch for ${asset}" >&2
  exit 1
fi

mapfile -t entries < <(tar -tzf "${tmpdir}/${asset}")
if (( ${#entries[@]} != 1 )) || [[ "${entries[0]#./}" != "${BINARY_NAME}" && "${entries[0]#./}" != "${BINARY_NAME}.exe" ]]; then
  echo "error: archive must contain exactly one rapprise binary" >&2
  exit 1
fi
tar -xzf "${tmpdir}/${asset}" -C "${tmpdir}"
binary="${tmpdir}/${entries[0]#./}"
[[ -f "${binary}" && ! -L "${binary}" ]] || { echo "error: archive binary is not a regular file" >&2; exit 1; }

mkdir -p "${INSTALL_DIR}"
[[ -w "${INSTALL_DIR}" ]] || { echo "error: install dir is not writable: ${INSTALL_DIR}" >&2; exit 1; }
install_tmp="$(mktemp "${INSTALL_DIR}/.${BINARY_NAME}.tmp.XXXXXX")"
install -m 755 "${binary}" "${install_tmp}"
mv -f "${install_tmp}" "${INSTALL_DIR}/${BINARY_NAME}"
install_tmp=""
echo "Installed ${BINARY_NAME} ${VERSION} to ${INSTALL_DIR}/${BINARY_NAME}"
