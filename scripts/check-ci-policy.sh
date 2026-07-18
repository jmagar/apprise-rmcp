#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

python3 <<'PY'
import datetime, json, re
from pathlib import Path

errors = []
workflow_dir = Path(".github/workflows")
for workflow in sorted(workflow_dir.glob("*.yml")):
    text = workflow.read_text()
    for line_no, line in enumerate(text.splitlines(), 1):
        match = re.search(r"\buses:\s*([^\s]+)", line)
        if not match or match.group(1).startswith("./"):
            continue
        ref = match.group(1).rsplit("@", 1)[-1]
        if not re.fullmatch(r"[0-9a-f]{40}", ref):
            errors.append(f"{workflow}:{line_no}: external Action is not pinned to a full commit SHA")

docker = (workflow_dir / "docker-publish.yml").read_text()
for required in ["workflow_run:", "exit-code: \"1\"", "actions/attest-sbom@"]:
    if required not in docker:
        errors.append(f"docker-publish.yml: missing policy marker {required!r}")
scan = docker.find("Blocking vulnerability scan")
login = docker.find("Log in only after")
push = docker.find("Push the exact scanned image")
if not (0 <= scan < login < push):
    errors.append("docker-publish.yml: scan must precede registry login and push")

release = (workflow_dir / "release.yml").read_text()
preflight = release.find("preflight:")
build = release.find("build:")
if preflight < 0 or build < preflight or "needs: [release-meta, preflight]" not in release:
    errors.append("release.yml: build/publish jobs must depend on complete preflight")
if "NODE_AUTH_TOKEN" in release or "NPM_TOKEN" in release:
    errors.append("release.yml: npm publication must use trusted publishing, not a long-lived token")

openwiki = (workflow_dir / "openwiki-update.yml").read_text()
if "npm ci --ignore-scripts" not in openwiki or "needs: prepare-toolchain" not in openwiki:
    errors.append("openwiki-update.yml: OpenWiki must be locked and prepared in an isolated job")
update_body = openwiki.split("  update:", 1)[-1]
if re.search(r"\bnpm\s+(install|ci)\b", update_body):
    errors.append("openwiki-update.yml: secret/private-network job must not install npm code")
lock = json.loads(Path("scripts/openwiki/package-lock.json").read_text())
if lock.get("packages", {}).get("", {}).get("dependencies", {}).get("openwiki") != "0.2.0":
    errors.append("scripts/openwiki/package-lock.json: openwiki must remain exactly pinned")

audit = Path(".cargo/audit.toml").read_text()
expiry = re.search(r"expires (\d{4}-\d{2}-\d{2})", audit)
if "RUSTSEC-2023-0071" in audit and (not expiry or datetime.date.today() > datetime.date.fromisoformat(expiry.group(1))):
    errors.append(".cargo/audit.toml: RSA advisory exception is missing an active expiry")

for installer in [Path("scripts/install.sh"), Path("packages/apprise-rmcp/scripts/install.js")]:
    text = installer.read_text()
    for marker in ["sha256", "timeout", "redirect"]:
        if marker.lower() not in text.lower():
            errors.append(f"{installer}: missing installer trust marker {marker}")

if errors:
    raise SystemExit("\n".join(f"ERROR: {error}" for error in errors))
print("CI supply-chain policy checks passed")
PY
