# apprise-rmcp contract inventory

## Public surfaces

| Surface | Contract |
|---|---|
| Repository/package | `apprise-rmcp` |
| Rust crate/service | `apprise-mcp` |
| Executable | `rapprise` |
| MCP tool | `apprise` |
| MCP HTTP/liveness | `POST /mcp`, `GET /health`, port `40050` |
| Upstream default | `http://localhost:8000` |
| Registry identity | `ai.dinglebear/apprise-rmcp` |

## MCP actions

| Action | Required | Optional | Effect |
|---|---|---|---|
| `notify` | `body` | `tag`, `title`, `type` | Send to tag, or all when omitted |
| `notify_url` | `urls`, `body` | `title`, `type` | One-off Apprise URLs |
| `health` | none | none | Check upstream health |
| `help` | none | none | Inline help |

Upstream JSON remains structured JSON; non-JSON success is returned as text.
There is no guaranteed synthetic `{"ok":true,"response":...}` wrapper.

## Canonical configuration inventory

Environment overrides TOML. Secrets belong in process env or
`${APPRISE_HOME:-~/.apprise}/.env` (`/data/.env` in containers).

| TOML field | Environment | Default | Sensitive | Mode | Runtime support |
|---|---|---|---:|---|---|
| `apprise.url` | `APPRISE_URL` | `http://localhost:8000` | no | all | supported |
| `apprise.token` | `APPRISE_TOKEN` | empty | yes | all | supported, outbound |
| `mcp.host` | `APPRISE_MCP_HOST` | `0.0.0.0` | no | HTTP | supported |
| `mcp.port` | `APPRISE_MCP_PORT` | `40050` | no | HTTP | supported |
| `mcp.server_name` | none | `apprise-mcp` | no | all | supported |
| `mcp.no_auth` | `APPRISE_MCP_NO_AUTH` | false | no | HTTP | loopback only |
| `mcp.api_token` | `APPRISE_MCP_TOKEN` | empty | yes | bearer | supported |
| `mcp.allowed_hosts` | `APPRISE_MCP_ALLOWED_HOSTS` | empty | no | HTTP | supported |
| `mcp.allowed_origins` | `APPRISE_MCP_ALLOWED_ORIGINS` | empty | no | HTTP | supported |
| `mcp.auth.mode` | `APPRISE_MCP_AUTH_MODE` | bearer | no | HTTP | bearer/oauth |
| `mcp.auth.public_url` | `APPRISE_MCP_PUBLIC_URL` | empty | no | OAuth | required |
| `mcp.auth.google_client_id` | `APPRISE_MCP_GOOGLE_CLIENT_ID` | empty | yes | OAuth | required |
| `mcp.auth.google_client_secret` | `APPRISE_MCP_GOOGLE_CLIENT_SECRET` | empty | yes | OAuth | required |
| `mcp.auth.admin_email` | `APPRISE_MCP_AUTH_ADMIN_EMAIL` | empty | personal | OAuth | required |
| `mcp.auth.allowed_client_redirect_uris` | `APPRISE_MCP_AUTH_ALLOWED_REDIRECT_URIS` | empty | no | OAuth | supported |
| `mcp.auth.sqlite_path` | `APPRISE_MCP_AUTH_SQLITE_PATH` | data-dir `auth.db` | no | OAuth | supported |
| `mcp.auth.key_path` | `APPRISE_MCP_AUTH_KEY_PATH` | data-dir `auth-jwt.pem` | yes | OAuth | supported |
| `mcp.auth.allowed_emails` | `APPRISE_MCP_AUTH_ALLOWED_EMAILS` | empty | personal | OAuth | supported |
| `mcp.auth.access_token_ttl_secs` | `APPRISE_MCP_AUTH_ACCESS_TOKEN_TTL_SECS` | 3600 | no | OAuth | supported |
| `mcp.auth.refresh_token_ttl_secs` | `APPRISE_MCP_AUTH_REFRESH_TOKEN_TTL_SECS` | 2592000 | no | OAuth | supported |
| `mcp.auth.auth_code_ttl_secs` | `APPRISE_MCP_AUTH_CODE_TTL_SECS` | 300 | no | OAuth | supported |
| `mcp.auth.register_rpm` | `APPRISE_MCP_AUTH_REGISTER_REQUESTS_PER_MINUTE` | 10 | no | OAuth | supported |
| `mcp.auth.authorize_rpm` | `APPRISE_MCP_AUTH_AUTHORIZE_REQUESTS_PER_MINUTE` | 60 | no | OAuth | supported |
| `mcp.auth.max_pending_oauth_states` | `APPRISE_MCP_AUTH_MAX_PENDING_OAUTH_STATES` | 1024 | no | OAuth | supported |
| `mcp.auth.disable_static_token_with_oauth` | `APPRISE_MCP_DISABLE_STATIC_TOKEN_WITH_OAUTH` | true | no | OAuth | supported |
| `apprise.max_concurrent_requests` | `APPRISE_MAX_CONCURRENT_REQUESTS` | 32 | no | all | supported |
| `apprise.max_response_bytes` | `APPRISE_MAX_RESPONSE_BYTES` | 65536 | no | all | supported |

`APPRISE_MCP_DISABLE_HTTP_AUTH` is a legacy no-auth alias. `APPRISE_HOME`
selects host data. `RUST_LOG` defaults to `info`.

## Authentication decision table

| Transport/bind | Settings | Required behavior |
|---|---|---|
| stdio | any HTTP auth | HTTP settings do not apply |
| HTTP loopback | `no_auth=true` | allowed for development |
| HTTP non-loopback | `no_auth=true` | reject startup |
| HTTP | bearer + token | require exact bearer token |
| HTTP | bearer without token | reject outside explicit loopback development |
| HTTP | OAuth | require OAuth/JWT plus issuer/client/admin state |
| HTTP | OAuth + static disable true | static MCP token must not bypass OAuth |

TLS is required at the reverse proxy for network exposure. A proxy is not a
substitute for the process bind/auth invariant.

## Installation and support

| Path | Supported hosts | Trust status |
|---|---|---|
| Source | Rust-supported hosts | auditable local build |
| npm/npx | Linux x86_64, Windows x86_64 | verifies published SHA-256, atomic install |
| `scripts/install.sh` | Linux x86_64 | verifies published SHA-256 |
| Plugin | platform of bundled binary | build with `just plugin-build`; stdio only |
| Container | image-supported Linux | pin immutable version/digest |

macOS and Linux ARM64 are not mapped by the npm launcher.
Releases publish GitHub build-provenance attestations; installers do not
automatically verify those attestations.

## Version model

Crate, npm package, registry package, `server.json`, release manifest, Git tag,
and assets are coupled. Release Please updates them together.

## Temporary security exception

`RUSTSEC-2023-0071` is currently accepted only because it is inherited through
`lab-auth`/`jsonwebtoken` and no compatible fixed RSA release is available.
This exception expires before the next production release or 2026-08-18,
whichever comes first. Until replacement, keep OAuth registration/authorization
rate limits enabled, restrict allowed accounts and redirect URIs, and monitor
auth failures. A release must not silently extend this acceptance.
