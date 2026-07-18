# apprise-rmcp

`apprise-rmcp` is a Rust MCP server and CLI for sending notifications through
an [Apprise API](https://github.com/caronc/apprise-api) server.

It exposes one MCP tool, `apprise`, plus the `rapprise` CLI. Agents can send
tagged notifications through a preconfigured Apprise server, run one-off
Apprise URL sends, and check upstream health through stdio MCP, Streamable HTTP
MCP, or direct shell commands.

**30-second path:** run `npx -y apprise-rmcp health --json` -> start loopback
HTTP with `APPRISE_MCP_HOST=127.0.0.1 npx -y apprise-rmcp serve` -> call
`tools/call` with `{"action":"health"}`.

**Status:** operational RMCP upstream-client server. Write-capable; notification
sends are intentional side effects. HTTP MCP supports loopback dev mode, static
bearer tokens, and Google OAuth through `lab-auth`.

**Not for:** replacing Apprise API, storing notification destinations in this
repo, scheduling reminders, building a generic webhook relay, multi-tenant
isolation, or passing upstream Apprise bearer tokens through MCP tool arguments.

## Contents

- [Naming](#naming)
- [Capabilities And Boundaries](#capabilities-and-boundaries)
- [Install](#install)
- [Quickstart](#quickstart)
- [Client Configuration](#client-configuration)
- [Runtime Surfaces](#runtime-surfaces)
- [MCP Tool Reference](#mcp-tool-reference)
- [CLI Reference](#cli-reference)
- [Configuration](#configuration)
- [Authentication](#authentication)
- [Safety And Trust Model](#safety-and-trust-model)
- [Architecture](#architecture)
- [Distribution Contract](#distribution-contract)
- [Development](#development)
- [Verification](#verification)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)
- [Related Servers](#related-servers)
- [Documentation](#documentation)
- [License](#license)

## Naming

| Surface | This repo |
|---|---|
| Repository | `apprise-rmcp` |
| Rust crate | `apprise-mcp` |
| Binary / CLI | `rapprise` |
| npm package | `apprise-rmcp` |
| npm binary aliases | `apprise-rmcp`, `rapprise` |
| MCP tool | `apprise` |
| Config home | `~/.apprise` on hosts, `/data` in containers |
| Env prefixes | `APPRISE_*`, `APPRISE_MCP_*`, `APPRISE_RMCP_*` for npm launcher controls |

The repo and npm package use the RMCP family name, while the shipped binary uses
the short Rust CLI name `rapprise`.

## Capabilities And Boundaries

- Send notifications through tags configured in the upstream Apprise API server.
- Send one-off notifications to Apprise URL schemas with `notify_url`.
- Check upstream Apprise API health.
- Expose the `send_alert` prompt for critical alert workflows.
- Provide setup and doctor commands for local plugin/runtime checks.

| This repo owns | Apprise owns | Explicitly out of scope |
|---|---|---|
| MCP/CLI projection, request validation, auth policy, response shaping, setup checks, and transport wiring. | Notification delivery, configured destinations, tags, delivery backend credentials, upstream API semantics. | Destination storage, scheduling, retry policy beyond upstream behavior, multi-tenant sandboxing, arbitrary webhook relay behavior. |

## Install

| Path | Command | Best for | Notes |
|---|---|---|---|
| npm / npx | `npx -y apprise-rmcp --help` | Linux/Windows x86_64 clients. | Verifies the release archive SHA-256 before atomic install. |
| Release installer | `curl -fsSL https://raw.githubusercontent.com/jmagar/apprise-rmcp/main/scripts/install.sh \| bash` | Linux x86_64 without Node. | Verifies the published SHA-256 before install. |
| Docker / Compose | `docker compose up -d` | Shared HTTP MCP deployments. | Reads `.env` and exposes container port `40050`. |
| Build from source | `cargo build --release` | Development and audits. | Produces `target/release/rapprise`. |
| Plugin | `just plugin-build && claude plugin install ./plugins/apprise` | Claude Code from this checkout. | Bundled `rapprise` stdio plugin. |

Releases publish SHA-256 files and GitHub build-provenance attestations. The
installers enforce the checksum but do not automatically verify the attestation;
high-assurance installs should verify it independently or build from source.
macOS and ARM64 are not currently mapped by the npm launcher.

### npm / npx

Run the stdio MCP server or CLI without a manual binary install:

```bash
npx -y apprise-rmcp --help
npx -y apprise-rmcp mcp
npx -y apprise-rmcp health --json
```

The npm package downloads `rapprise` during `postinstall`. Override download
behavior only when testing packaging:

| Variable | Purpose |
|---|---|
| `APPRISE_RMCP_SKIP_DOWNLOAD=1` | Skip postinstall binary download. |
| `APPRISE_RMCP_VERSION` or `APPRISE_RMCP_BINARY_VERSION` | Select the GitHub Release tag. |
| `APPRISE_RMCP_REPO` | Select the GitHub repo used for release downloads. |
| `APPRISE_RMCP_RELEASE_BASE_URL` | Select a custom release base URL. |

### Build From Source

```bash
git clone https://github.com/jmagar/apprise-rmcp
cd apprise-rmcp
cargo build --release
./target/release/rapprise --help
```

Minimum supported Rust version: 1.86.

## Quickstart

### 1. Start Or Point At Apprise API

The default upstream is `http://localhost:8000`.

```bash
docker run --rm -p 8000:8000 caronc/apprise:latest
```

Use `APPRISE_URL` when the API server is elsewhere:

```bash
export APPRISE_URL=http://100.120.242.29:8766
```

Set `APPRISE_TOKEN` only when your Apprise API server requires bearer auth:

```bash
export APPRISE_TOKEN=...
```

### 2. Run A Safe CLI Call

```bash
npx -y apprise-rmcp health --json
```

### 3. Start Loopback HTTP MCP

```bash
APPRISE_MCP_HOST=127.0.0.1 npx -y apprise-rmcp serve
```

In another shell:

```bash
curl -sf http://127.0.0.1:40050/health
```

### 4. Make A First MCP Call

```bash
curl -s -X POST http://127.0.0.1:40050/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"apprise","arguments":{"action":"health"}}}'
```

## Client Configuration

### Claude Code Stdio

```json
{
  "mcpServers": {
    "apprise": {
      "command": "npx",
      "args": ["-y", "apprise-rmcp", "mcp"],
      "env": {
        "APPRISE_URL": "http://localhost:8000"
      }
    }
  }
}
```

### Claude Code HTTP

```json
{
  "mcpServers": {
    "apprise": {
      "type": "http",
      "url": "http://127.0.0.1:40050/mcp",
      "headers": {
        "Authorization": "Bearer ${APPRISE_MCP_TOKEN}"
      }
    }
  }
}
```

### Codex / Labby Gateway

Register Apprise through Labby as an HTTP upstream when sharing one long-running
server, or run it directly as stdio for local-only use.

```toml
[mcp_servers.apprise]
command = "npx"
args = ["-y", "apprise-rmcp", "mcp"]
```

### Generic MCP JSON

```json
{
  "command": "rapprise",
  "args": ["mcp"],
  "env": {
    "APPRISE_URL": "http://localhost:8000"
  }
}
```

Do not put `APPRISE_TOKEN`, OAuth secrets, SSH keys, passwords, or upstream
bearer tokens in MCP tool arguments. Use env, config files, or the MCP client's
secret storage.

## Runtime Surfaces

| Surface | Status | Entry point | Purpose |
|---|---:|---|---|
| MCP stdio | Supported | `rapprise mcp`, `npx -y apprise-rmcp mcp` | Local child-process MCP clients. |
| MCP HTTP | Supported | `rapprise serve`, `POST /mcp` | Streamable HTTP MCP for local or shared server deployments. |
| CLI | Supported | `rapprise <command>` | Scriptable parity and debugging. |
| Prompt | Supported | `send_alert` | Reusable critical-alert workflow. |
| REST API | Not shipped | N/A | Apprise API already owns the REST API. |
| Web UI | Not shipped | N/A | Apprise API already owns the web UI. |

## MCP Tool Reference

One MCP tool is exposed: `apprise`. Pass the required `action` argument to select
the operation.

### Read Actions

| Action | Description | Required params | Optional params |
|---|---|---|---|
| `health` | Check Apprise API server health. | none | none |
| `help` | Return built-in markdown tool help. | none | none |

### Write Actions

| Action | Description | Required params | Optional params |
|---|---|---|---|
| `notify` | Send to a configured Apprise tag, or all configured services when `tag` is omitted. | `body` | `tag`, `title`, `type` |
| `notify_url` | Send a stateless one-off notification to one or more Apprise URL schemas. | `urls`, `body` | `title`, `type` |

### Notification Types

| Type | Meaning |
|---|---|
| `info` | Informational notification. |
| `success` | Successful operation. |
| `warning` | Non-critical warning. |
| `failure` | Critical failure or error. |

### MCP Call Examples

```bash
curl -s -X POST http://127.0.0.1:40050/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"apprise","arguments":{"action":"health"}}}'
```

```json
{
  "name": "apprise",
  "arguments": {
    "action": "notify",
    "body": "Deployment succeeded",
    "tag": "ops",
    "title": "Deploy complete",
    "type": "success"
  }
}
```

Curated action summaries live here. `docs/INVENTORY.md` is the current
source of truth for complete parameters until a generated `docs/MCP_SCHEMA.md`
is added.

## CLI Reference

The CLI calls the same service methods as the MCP tool.

```bash
rapprise health [--json]
rapprise notify <body> [--tag TAG] [--title T] [--type info|success|warning|failure] [--json]
rapprise notify-url <urls> <body> [--title T] [--type info|success|warning|failure] [--json]

rapprise serve
rapprise serve mcp
rapprise mcp
rapprise doctor [--json]
rapprise setup check
rapprise setup repair
rapprise setup plugin-hook [--no-repair]
```

## Configuration

Configuration loads from `config.toml` when present, then environment variables
override those values. On startup it loads
`${APPRISE_HOME:-~/.apprise}/.env` on hosts or `/data/.env` in containers.
See [the complete inventory](docs/INVENTORY.md).

### Upstream Variables

| Variable | Required | Description |
|---|---:|---|
| `APPRISE_URL` | no | Apprise API server base URL. Defaults to `http://localhost:8000`. |
| `APPRISE_TOKEN` | only for protected upstreams | Optional upstream Apprise API bearer token. |

### Runtime Variables

| Variable | Default | Description |
|---|---|---|
| `APPRISE_MCP_HOST` | `0.0.0.0` | HTTP MCP bind host. |
| `APPRISE_MCP_PORT` | `40050` | HTTP MCP bind port. |
| `APPRISE_MCP_TOKEN` | empty | Static bearer token for HTTP MCP when not in loopback dev mode. |
| `APPRISE_MCP_NO_AUTH` | `false` | Disable HTTP MCP auth. Use only on loopback or behind a trusted gateway. |
| `APPRISE_MCP_AUTH_MODE` | `bearer` | Set to `oauth` for Google OAuth through `lab-auth`. |
| `APPRISE_MCP_PUBLIC_URL` | empty | Public URL for OAuth metadata and protected-resource discovery. |
| `APPRISE_MCP_GOOGLE_CLIENT_ID` | empty | Google OAuth client ID. |
| `APPRISE_MCP_GOOGLE_CLIENT_SECRET` | empty | Google OAuth client secret. |
| `APPRISE_MCP_AUTH_ADMIN_EMAIL` | empty | Initial/admin OAuth email. |
| `APPRISE_MCP_ALLOWED_HOSTS` | empty | Additional accepted HTTP Host values. |
| `APPRISE_MCP_ALLOWED_ORIGINS` | empty | Additional CORS origins for HTTP MCP. |
| `APPRISE_MCP_AUTH_ALLOWED_REDIRECT_URIS` | empty | OAuth client redirect URIs. |
| `RUST_LOG` | `info` | Rust log filter. Stdio logs must stay off stdout. |

## Authentication

| Policy | When | Effect |
|---|---|---|
| Stdio | `rapprise mcp` | Local process trust; HTTP auth does not apply. |
| Loopback dev | loopback plus no-auth | Permits unauthenticated local HTTP. |
| Non-loopback no-auth | non-loopback plus no-auth | Invalid; startup must reject it. |
| Static bearer | bearer plus `APPRISE_MCP_TOKEN` | Require exact bearer token. |
| OAuth | issuer/client/admin settings | Require OAuth/JWT. |
| OAuth static control | disable-static true | Static token must not bypass OAuth. |

MCP scopes are `apprise:notify` and `apprise:admin`. OAuth tokens are checked
before MCP calls are dispatched.

## Safety And Trust Model

- MCP callers never provide `APPRISE_TOKEN`, OAuth secrets, static bearer tokens,
  passwords, SSH keys, or API keys as tool arguments.
- Upstream Apprise credentials are loaded from env/config only.
- `notify` is the preferred path because destinations are configured upstream
  under tags.
- `notify_url` intentionally accepts Apprise URL schemas in MCP arguments for
  one-off sends. Treat those URLs as sensitive payloads and avoid using them
  when a tagged upstream destination is available.
- Apprise API is the durable source of destination configuration; this server is
  a thin projection over that API.
- HTTP mode should not be exposed beyond loopback without bearer or OAuth auth
  plus TLS from an upstream reverse proxy.

## Architecture

```text
MCP client / CLI
       |
       v
rapprise
       |
       +-- MCP shim: JSON args -> AppriseService -> structured result
       +-- CLI shim: argv -> AppriseService -> stdout
       |
       v
AppriseService
       |
       v
AppriseClient
       |
       v
Apprise API server
       |
       v
Notification backends
```

| Path | Role |
|---|---|
| `src/app.rs` | Business service layer, response shaping, counters, and notification calls. |
| `src/apprise.rs` | Apprise API REST client. |
| `src/mcp/` | RMCP tool, prompt, schema, routes, and auth checks. |
| `src/cli.rs` | CLI parser, doctor/setup helpers, and output formatting. |
| `src/config.rs` | Env/config loading and defaults. |
| `packages/apprise-rmcp/` | npm launcher and release-binary downloader. |

Notification commands and MCP converge on `AppriseService`. The CLI also owns
setup, doctor, self-install, filesystem, and output orchestration today; it is
not a pure argument shim.

## Distribution Contract

| Artifact | File(s) | Must align with |
|---|---|---|
| Rust crate/binary | `Cargo.toml`, `Cargo.lock` | Git tag, release assets, CLI docs, install scripts. |
| npm launcher | `packages/apprise-rmcp/package.json`, `bin/rapprise.js`, `lib/platform.js`, `scripts/install.js` | GitHub Release tag and assets named `rapprise-x86_64.tar.gz` and `rapprise-windows-x86_64.tar.gz`. |
| GitHub Releases | `.github/workflows/*`, `scripts/install.sh`, `install.sh` | Package version, binary name, checksums, supported platforms. |
| Docker / Compose | `Dockerfile`, `docker-compose*.yml` | Exposed port `40050`, healthcheck `/health`, env file contract. |
| MCP registry | `server.json` | Identity `ai.dinglebear/apprise-rmcp`, stdio package, version. |
| Plugin | `plugins/apprise` | Bundled `rapprise` stdio and direct setup hook. |
| Docs | `README.md`, `docs/INVENTORY.md`, `docs/QUICKSTART.md` | Current binary name, default port, action list, and env names. |

Release invariant: npm, crate, registry/server metadata, manifest, GitHub tag,
and native assets move together. Release Please owns these updates.

## Development

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
npm --prefix packages/apprise-rmcp run check
```

## Verification

```bash
# Binary and CLI
cargo build --release
./target/release/rapprise --version
./target/release/rapprise health --json

# HTTP health
APPRISE_MCP_HOST=127.0.0.1 ./target/release/rapprise serve
curl -sf http://127.0.0.1:40050/health

# MCP tool call
curl -s -X POST http://127.0.0.1:40050/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"apprise","arguments":{"action":"health"}}}'
```

For live notification tests, configure at least one destination in the Apprise
API server and call `notify` with its tag.

## Deployment

### Docker / Compose

```bash
cp .env.example .env
$EDITOR .env
docker compose up -d
curl -sf http://127.0.0.1:40050/health
```

The container stores app data under `/data`, normally mounted from
`${HOME}/.apprise`.

### Reverse Proxy

Expose only `/mcp` and `/health`. Preserve Streamable HTTP headers, require TLS,
and configure bearer or OAuth auth before exposing the server beyond loopback.

### Plugin

```bash
claude plugin install plugins/apprise
rapprise setup check
```

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `401` from `/mcp` | Missing or wrong bearer/OAuth token. | Check `APPRISE_MCP_TOKEN` and client headers, or use loopback dev mode locally. |
| CLI health fails | Apprise API is not reachable. | Check `APPRISE_URL` and the upstream Apprise API server. |
| `notify` sends nowhere | No upstream destinations match the tag. | Check configured tags in Apprise API or omit `tag` to send to all configured services. |
| `notify_url` fails | Invalid Apprise URL schema or blocked destination backend. | Test the URL with Apprise API directly and prefer configured tags for repeated use. |
| stdio MCP JSON parse errors | Logs went to stdout. | Keep protocol logs off stdout and lower `RUST_LOG` if needed. |
| npm launcher cannot find binary | Release asset download failed or was skipped. | Reinstall, check `APPRISE_RMCP_VERSION`, or build `rapprise` from source. |

## Related Servers

- `unifi-rmcp / rustifi` - UniFi controller REST API bridge.
- `tailscale-rmcp / rustscale` - Tailscale API bridge for devices, users, and tailnet operations.
- `unraid-rmcp / unrust` - Unraid GraphQL bridge for NAS and server management.
- `gotify-rmcp` - Gotify push notification bridge for sends, messages, apps, and clients.
- `arcane-rmcp` - Arcane Docker management bridge for containers and related resources.
- `yarr-rmcp` - Media-stack bridge for Sonarr, Radarr, Prowlarr, Plex, and related services.
- `ytdl-mcp` - Media download and metadata workflow server.
- `synapse` - Local Synapse workflow server for scout and flux actions.
- `cortex` - Syslog and homelab log aggregation MCP server.
- `axon` - RAG, crawl, scrape, extract, and semantic search project.
- `lab` - Homelab control plane and Labby gateway project.
- `lumen` - Local semantic code search MCP server.
- `nugs` - Project/package management helper for local agent workflows.
- `agentcast` - Agent transcript and activity publishing project.
- `soma` - RMCP scaffold/runtime template for new provider-backed servers.

## Documentation

Start here:

- [`docs/QUICKSTART.md`](docs/QUICKSTART.md) - focused setup flow.
- [`docs/INVENTORY.md`](docs/INVENTORY.md) - component inventory for actions,
  CLI commands, env vars, and endpoints.
- [`docs/README.md`](docs/README.md) - docs index.
- [`server.json`](server.json) - MCP registry metadata.
- [`packages/apprise-rmcp/README.md`](packages/apprise-rmcp/README.md) - npm
  package launcher notes.

This README is curated. Generated or exhaustive catalogs should be refreshed in
their own files and treated as the source of truth for current branch details.

## License

MIT
