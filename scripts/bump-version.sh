#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="${1:-}"
[[ "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]] || {
  echo "usage: $0 <semver>" >&2
  exit 2
}
python3 - "${repo_root}" "${version}" <<'PY'
import json, re, sys
from pathlib import Path
root, version = Path(sys.argv[1]), sys.argv[2]
cargo_path = root / "Cargo.toml"
cargo = cargo_path.read_text()
cargo, count = re.subn(r"(?ms)(^\[package\].*?^version\s*=\s*)\"[^\"]+\"", rf'\g<1>"{version}"', cargo, count=1)
if count != 1: raise SystemExit("unable to update Cargo.toml package.version")
cargo_path.write_text(cargo)
for relative in ["packages/apprise-rmcp/package.json", "server.json"]:
    path = root / relative
    data = json.loads(path.read_text())
    data["version"] = version
    if relative.endswith("package.json"): data["binaryVersion"] = version
    for package in data.get("packages", []):
        if "version" in package: package["version"] = version
    if relative == "server.json":
        data["_meta"]["io.modelcontextprotocol.registry/publisher-provided"]["buildInfo"]["version"] = version
    path.write_text(json.dumps(data, indent=2) + "\n")
manifest_path = root / ".release-please-manifest.json"
manifest = json.loads(manifest_path.read_text()); manifest["."] = version
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
PY
"${repo_root}/scripts/check-version-sync.sh"
