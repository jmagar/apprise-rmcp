# apprise-rmcp

MCP server and CLI for [Apprise](https://github.com/caronc/apprise) — a universal push notification library that supports 80+ services (Slack, Discord, PagerDuty, Gotify, ntfy, Telegram, email, and many more).


## npm / npx

Run the stdio MCP server or CLI without a manual binary install:

```bash
npx -y apprise-rmcp --help
```

MCP clients can use the same launcher:

```json
{
  "mcpServers": {
    "apprise-rmcp": {
      "command": "npx",
      "args": ["-y", "apprise-rmcp"]
    }
  }
}
```

The npm package downloads the `rapprise` binary from GitHub Releases during `postinstall` and keeps the release tag aligned with `packages/apprise-rmcp/package.json`.

Naming follows the rmcp family convention: the repo is `<service>-rmcp`, the npm package is `<service>-rmcp`, and the installed CLI/binary is `r<service>`. For this service that means repo/package `apprise-rmcp` and CLI `rapprise`.

## What it does

`apprise-rmcp` bridges Claude (and any MCP client) to an Apprise API server so that AI agents can send push notifications as part of their workflows — alerts, status updates, job completions, incident reports.

## Architecture

```
Claude / MCP client
        |
   apprise-rmcp (this server)
        |  HTTP REST
   Apprise API server   (http://your-host:8766)
        |
   80+ notification services
   (Slack, Discord, email, ntfy, Gotify, ...)
```

## Quickstart

See [docs/QUICKSTART.md](docs/QUICKSTART.md) for a 5-minute setup guide.

## MCP tool: `apprise`

The server exposes a single `apprise` tool with an `action` selector.

### `notify` — send to configured tag (or all services)

```json
{
  "action": "notify",
  "body": "Deployment succeeded",
  "tag": "ops",
  "title": "Deploy complete",
  "type": "success"
}
```

`tag` is optional. Omit it to broadcast to all configured services.

### `notify_url` — stateless one-off notification

```json
{
  "action": "notify_url",
  "urls": "slack://tokenA/tokenB/tokenC",
  "body": "Critical error in prod",
  "title": "ALERT",
  "type": "failure"
}
```

No pre-configuration on the Apprise server required.

### `health` — server health check

```json
{ "action": "health" }
```

### `help` — inline documentation

```json
{ "action": "help" }
```

## Notification types

| type | meaning |
|------|---------|
| `info` | Informational (default) |
| `success` | Successful operation |
| `warning` | Non-critical warning |
| `failure` | Critical failure / error |

## CLI

```bash
# Send to all services under the "ops" tag
apprise notify "Backup finished" --tag ops --title "Backup" --type success

# Stateless one-off (no server pre-configuration needed)
apprise notify-url "slack://tokenA/tokenB/tokenC" "Hello from CLI"

# Health check
apprise health

# Raw JSON output
apprise health --json

# MCP modes
apprise serve      # HTTP MCP server (default)
rapprise mcp       # stdio MCP transport
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `APPRISE_URL` | `http://localhost:8000` | Apprise API server URL |
| `APPRISE_TOKEN` | _(empty)_ | API token (optional for open installs) |
| `APPRISE_MCP_HOST` | `0.0.0.0` | MCP HTTP bind host |
| `APPRISE_MCP_PORT` | `8765` | MCP HTTP bind port |
| `APPRISE_MCP_TOKEN` | _(none)_ | Static bearer token for MCP HTTP auth |
| `RUST_LOG` | `info` | Log filter |

## Claude Desktop config

```json
{
  "mcpServers": {
    "apprise": {
      "command": "apprise",
      "args": ["mcp"],
      "env": {
        "APPRISE_URL": "http://100.120.242.29:8766"
      }
    }
  }
}
```

## Building

```bash
cargo build --release
# Binary: target/release/apprise
```

Minimum Rust version: **1.86**

## Apprise API server

This tool connects to a running [Apprise API](https://github.com/caronc/apprise-api) server, not the Python library directly.

Quick start with Docker:

```bash
docker run -p 8000:8000 caronc/apprise:latest
```

The API server lets you pre-configure notification services under named tags via its web UI or REST API, then notify them by tag from `apprise-rmcp`.
