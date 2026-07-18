#!/usr/bin/env bash
set -euo pipefail

repo_root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
expected="${EXPECTED_VERSION:-}"
cd "${repo_root}"

python3 - "${expected}" <<'PY'
import json, re, sys
from pathlib import Path

expected = sys.argv[1].removeprefix("v")
values = {}

cargo = Path("Cargo.toml").read_text()
match = re.search(r"(?ms)^\[package\].*?^version\s*=\s*\"([^\"]+)\"", cargo)
if not match:
    raise SystemExit("[version-sync] Cargo.toml package version is missing")
values["Cargo.toml package.version"] = match.group(1)

npm = json.loads(Path("packages/apprise-rmcp/package.json").read_text())
values["packages/apprise-rmcp/package.json version"] = npm["version"]
values["packages/apprise-rmcp/package.json binaryVersion"] = npm["binaryVersion"]

server = json.loads(Path("server.json").read_text())
values["server.json version"] = server["version"]
for index, package in enumerate(server.get("packages", [])):
    if "version" in package:
        values[f"server.json packages[{index}].version"] = package["version"]
values["server.json buildInfo.version"] = server["_meta"]["io.modelcontextprotocol.registry/publisher-provided"]["buildInfo"]["version"]

manifest = json.loads(Path(".release-please-manifest.json").read_text())
values[".release-please-manifest.json root"] = manifest["."]

canonical = expected or values["Cargo.toml package.version"]
mismatches = {name: value for name, value in values.items() if value != canonical}
if mismatches:
    print(f"[version-sync] FAIL: expected every release artifact at {canonical}", file=sys.stderr)
    for name, value in values.items():
        print(f"  {'!' if name in mismatches else ' '} {name}: {value}", file=sys.stderr)
    raise SystemExit(1)
print(f"[version-sync] OK: {len(values)} release fields at v{canonical}")
PY
