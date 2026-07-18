#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
compose_file="${APPRISE_MCP_COMPOSE_FILE:-${repo_root}/docker-compose.prod.yml}"
state_dir="${APPRISE_HOME:-${HOME}/.apprise}"
state_file="${APPRISE_MCP_DEPLOY_STATE:-${state_dir}/deployment-state}"
service="apprise-mcp"

valid_image() {
  [[ "$1" =~ ^[a-zA-Z0-9._/-]+(:[a-zA-Z0-9._-]+)?@sha256:[0-9a-f]{64}$ ]]
}

read_state() {
  local key="$1"
  [[ -f "${state_file}" ]] || return 1
  sed -n "s/^${key}=//p" "${state_file}" | head -1
}

wait_healthy() {
  local cid status
  cid="$(APPRISE_MCP_IMAGE="$1" docker compose -f "${compose_file}" ps -q "${service}")"
  [[ -n "$cid" ]] || return 1
  for _ in {1..30}; do
    status="$(docker inspect "$cid" --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}')"
    [[ "$status" == healthy ]] && return 0
    [[ "$status" == unhealthy || "$status" == exited || "$status" == dead ]] && return 1
    sleep 2
  done
  return 1
}

deploy() {
  local image="$1" previous="" temporary
  valid_image "$image" || { echo "error: image must be an immutable registry reference ending in @sha256:<64 hex>" >&2; exit 2; }
  previous="$(docker inspect "$service" --format '{{.Config.Image}}' 2>/dev/null || true)"
  APPRISE_MCP_IMAGE="$image" docker compose -f "$compose_file" pull "$service"
  APPRISE_MCP_IMAGE="$image" docker compose -f "$compose_file" up -d --no-build --force-recreate "$service"
  if ! wait_healthy "$image"; then
    echo "error: deployment did not become healthy" >&2
    [[ -z "$previous" ]] || APPRISE_MCP_IMAGE="$previous" docker compose -f "$compose_file" up -d --no-build --force-recreate "$service"
    exit 1
  fi
  mkdir -p "$state_dir"
  chmod 700 "$state_dir"
  temporary="$(mktemp "${state_file}.tmp.XXXXXX")"
  { echo "CURRENT_IMAGE=$image"; echo "PREVIOUS_IMAGE=$previous"; } > "$temporary"
  chmod 600 "$temporary"
  mv -f "$temporary" "$state_file"
  echo "deployed $image"
}

case "${1:-}" in
  deploy) [[ $# == 2 ]] || { echo "usage: $0 deploy <image@sha256:digest>" >&2; exit 2; }; deploy "$2" ;;
  rollback)
    previous="$(read_state PREVIOUS_IMAGE || true)"
    valid_image "$previous" || { echo "error: no immutable previous image recorded in $state_file" >&2; exit 1; }
    deploy "$previous"
    ;;
  *) echo "usage: $0 deploy <image@sha256:digest> | rollback" >&2; exit 2 ;;
esac
