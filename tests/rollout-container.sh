#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/home"

cat > "$fixture/bin/docker" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
echo "${APPRISE_MCP_IMAGE:-} :: $*" >> "$DOCKER_LOG"
if [[ "$1 $2" == "inspect apprise-mcp" && "$*" == *".Config.Image"* ]]; then
  echo "registry.local:5000/apprise-mcp:latest"
elif [[ "$1 $2" == "inspect apprise-mcp" && "$*" == *".Image"* ]]; then
  echo "sha256:image-id"
elif [[ "$1 $2 $3" == "image inspect sha256:image-id" ]]; then
  echo "registry.local:5000/apprise-mcp@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
elif [[ "$1 $2" == "compose -f" && "$*" == *" ps -q "* ]]; then
  if [[ "${APPRISE_MCP_IMAGE:-}" == *"bbbbbbbbbbbbbbbb"* ]]; then echo target-cid; else echo previous-cid; fi
elif [[ "$1 $2" == "inspect target-cid" ]]; then
  echo unhealthy
elif [[ "$1 $2" == "inspect previous-cid" ]]; then
  echo healthy
fi
MOCK
chmod +x "$fixture/bin/docker"

export PATH="$fixture/bin:$PATH"
export HOME="$fixture/home"
export DOCKER_LOG="$fixture/docker.log"
export APPRISE_MCP_DEPLOY_STATE="$fixture/deployment-state"
export APPRISE_MCP_COMPOSE_FILE="$fixture/compose.yml"
: > "$APPRISE_MCP_COMPOSE_FILE"

target="registry.local:5000/apprise-mcp@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
if "$repo_root/scripts/rollout-container.sh" deploy "$target"; then
  echo "expected unhealthy target deployment to fail" >&2
  exit 1
fi

grep -F "registry.local:5000/apprise-mcp@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa :: compose" "$DOCKER_LOG" >/dev/null
grep -F "inspect previous-cid" "$DOCKER_LOG" >/dev/null

echo "rollout contract tests passed"
