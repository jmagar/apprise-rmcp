# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Changed

- Standardized package/repo identity on `apprise-rmcp`, executable paths on
  `rapprise`, HTTP MCP on `40050`, and registry identity on
  `ai.dinglebear/apprise-rmcp`.
- Defined one coupled version for crate, npm, registry, tag, and native assets.
- Defined the plugin as bundled stdio with direct bundled-binary setup hooks.
- Published canonical auth, configuration, platform, and installer-trust docs.

### Removed

- Removed unsupported plugin options and the stale tracked `.claude/plugins` copy.

## [0.1.1] — 2026-06-01

### Changed

- Plugin hooks call `${CLAUDE_PLUGIN_ROOT}/bin/rapprise setup plugin-hook`
  directly. Plugin data remains in the canonical resolver selected by
  `APPRISE_HOME`, `CLAUDE_PLUGIN_DATA`, or the host/container default; no
  data migration or copy is performed.

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
