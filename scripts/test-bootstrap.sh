#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/assets"
cp "$repo_root/install.sh" "$fixture/bootstrap.sh"
cat > "$fixture/assets/rapprise-installer.sh" <<'EOF'
#!/usr/bin/env bash
printf verified > "$BOOTSTRAP_RESULT"
EOF
sha256sum "$fixture/assets/rapprise-installer.sh" > "$fixture/assets/rapprise-installer.sh.sha256"
printf '{"verificationMaterial":{}}\n' > "$fixture/assets/rapprise-installer.sh.sigstore.json"

cat > "$fixture/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
destination=""
url=""
while (($#)); do
  case "$1" in
    -o) destination="$2"; shift 2 ;;
    http*) url="$1"; shift ;;
    *) shift ;;
  esac
done
cp "$BOOTSTRAP_ASSETS/${url##*/}" "$destination"
EOF
cat > "$fixture/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then echo 'gh version 2.68.0'; exit 0; fi
[[ "${GH_SHOULD_FAIL:-0}" != 1 ]]
EOF
chmod +x "$fixture/bin/curl" "$fixture/bin/gh"

export PATH="$fixture/bin:$PATH"
export BOOTSTRAP_ASSETS="$fixture/assets"
export BOOTSTRAP_RESULT="$fixture/result"
APPRISE_RMCP_VERSION=v9.9.9 bash "$fixture/bootstrap.sh"
[[ "$(cat "$BOOTSTRAP_RESULT")" == verified ]]

rm -f "$BOOTSTRAP_RESULT"
if GH_SHOULD_FAIL=1 APPRISE_RMCP_VERSION=v9.9.9 bash "$fixture/bootstrap.sh"; then
  echo "error: bootstrap accepted failed provenance verification" >&2
  exit 1
fi
[[ ! -e "$BOOTSTRAP_RESULT" ]]

echo "bootstrap trust tests passed"
