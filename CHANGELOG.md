# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1] — 2026-06-01

### Changed

- Plugin `SessionStart`/`ConfigChange` hooks now call `${CLAUDE_PLUGIN_ROOT}/bin/rapprise setup plugin-hook` directly instead of going through the `plugin-setup.sh` shell wrapper. The env-var mapping the script performed (`CLAUDE_PLUGIN_OPTION_*` → `APPRISE_*`) now lives in `apply_plugin_options()` in `src/cli.rs`, called before `Config::load()` in the setup branch of `main` (apprise is template-style: the setup check validates the pre-loaded config). The `CLAUDE_PLUGIN_DATA` → `APPRISE_HOME` re-export was dropped (redundant: `setup_data_dir()` reads `CLAUDE_PLUGIN_DATA` natively).

### Removed

- `plugins/apprise/hooks/plugin-setup.sh` — the wrapper was a pure env-mapping middleman now handled by the binary's `setup plugin-hook` command.

## [0.1.0] — 2026-05-13

### Added

- Initial release of `apprise-mcp`
- `AppriseClient` HTTP REST client for the Apprise API
  - `notify(tag, title, body, type)` — POST /notify/{tag}
  - `notify_all(title, body, type)` — POST /notify
  - `notify_url(urls, title, body, type)` — stateless POST /notify/
  - `health()` — GET /health
- `AppriseService` business logic layer wrapping the client
- MCP tool `apprise` with actions: `notify`, `notify_url`, `health`, `help`
- MCP prompt `send_alert` for guided critical alert sending
- CLI: `notify`, `notify-url`, `health` subcommands
- HTTP MCP server (axum + rmcp streamable HTTP transport)
- stdio MCP transport
- `NotifyType` enum: `info`, `success`, `warning`, `failure`
- Config loading from `config.toml` + environment variables
  - `APPRISE_URL`, `APPRISE_TOKEN`
  - `APPRISE_MCP_HOST`, `APPRISE_MCP_PORT`, `APPRISE_MCP_TOKEN`
- Integration tests: stub-based graceful failure tests for all service methods
- Unit tests: `NotifyType` parsing, config defaults, bind address
