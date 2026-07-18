#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

test -L AGENTS.md && test "$(readlink AGENTS.md)" = "CLAUDE.md"
test -L GEMINI.md && test "$(readlink GEMINI.md)" = "CLAUDE.md"

for file in server.json release-please-config.json .release-please-manifest.json \
  plugins/apprise/.claude-plugin/plugin.json \
  plugins/apprise/.codex-plugin/plugin.json plugins/apprise/.mcp.json \
  plugins/apprise/hooks/hooks.json; do
  jq -e . "$file" >/dev/null
done

test "$(jq -r .name server.json)" = "ai.dinglebear/apprise-rmcp"
test "$(jq -r .version server.json)" = "$(jq -r '.packages[0].version' server.json)"
test "$(jq -r .version server.json)" = "$(jq -r '."."' .release-please-manifest.json)"
test "$(jq -r .version server.json)" = "$(node -p 'require("./packages/apprise-rmcp/package.json").version')"
test "$(jq -r .version server.json)" = \
  "$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "apprise-mcp") | .version')"
test "$(jq -r .version server.json)" = \
  "$(jq -r '._meta["io.modelcontextprotocol.registry/publisher-provided"].buildInfo.version' server.json)"
test "$(jq -r 'has("userConfig")' plugins/apprise/.claude-plugin/plugin.json)" = false

if grep -R -n -E 'syslog-mcp|target/release/apprise|default 8765|localhost:8765|tv\.tootie/apprise-mcp' \
  README.md CLAUDE.md docs plugins .claude server.json; then
  echo "documentation contains stale product, binary, port, or registry text" >&2
  exit 1
fi

# This is the literal placeholder consumed by the plugin host.
# shellcheck disable=SC2016
expected_plugin_command='${CLAUDE_PLUGIN_ROOT}/bin/rapprise'
test "$(jq -r '.mcpServers.apprise.command' plugins/apprise/.mcp.json)" = \
  "$expected_plugin_command"
test -z "$(find .claude/plugins/apprise-mcp -type f -print 2>/dev/null)"

grep -F 'gh attestation verify rapprise-installer.sh' README.md >/dev/null
if grep -F 'raw.githubusercontent.com/jmagar/apprise-rmcp' README.md >/dev/null; then
  echo "README must not execute unverified raw installer content" >&2
  exit 1
fi
grep -F 'just build-plugin && claude plugin install' README.md >/dev/null
grep -F 'just build-plugin' docs/QUICKSTART.md >/dev/null
grep -E '^build-plugin:' Justfile >/dev/null
grep -F 'notify`, `notify_url`, `health`, `status`, `help' CLAUDE.md >/dev/null
grep -F 'APPRISE_MCP_IMAGE=ghcr.io/jmagar/apprise-mcp@sha256:' .env.example >/dev/null
if grep -F 'APPRISE_MCP_VERSION=' .env.example >/dev/null; then
  echo ".env.example contains obsolete APPRISE_MCP_VERSION" >&2
  exit 1
fi
grep -F 'GitHub CLI 2.68+' README.md docs/INVENTORY.md packages/apprise-rmcp/README.md >/dev/null
grep -F 'APPRISE_DATA_DIR=${HOME}/.apprise' .env.example >/dev/null
grep -F '${APPRISE_DATA_DIR:-${HOME}/.apprise}:/data' docker-compose.prod.yml >/dev/null
if grep -F '${APPRISE_HOME:-${HOME}/.apprise}:/data' docker-compose.prod.yml; then
  echo "Compose must not pass a host APPRISE_HOME path into the container" >&2
  exit 1
fi

echo "documentation and plugin contracts are consistent"
