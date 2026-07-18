# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0](https://github.com/jmagar/apprise-rmcp/compare/v0.1.3...v0.2.0) (2026-07-18)


### Added

* align apprise npm launcher naming ([9f28c5c](https://github.com/jmagar/apprise-rmcp/commit/9f28c5c0c75f2b6268132105103526d8d0b58cca))


### Fixed

* **ci:** allow multi-arch publication to finish ([fb73923](https://github.com/jmagar/apprise-rmcp/commit/fb739236923b80510d1b0de807a6d25f25e31c6a))
* **ci:** correct Docker QEMU action pin ([d0b38c5](https://github.com/jmagar/apprise-rmcp/commit/d0b38c53553427d6bf36979e20115174802eeecb))
* **ci:** switch OpenWiki to local openai-compatible proxy ([65fd327](https://github.com/jmagar/apprise-rmcp/commit/65fd32746f60c57896380a43e7cdbe8dbe0e35a3))
* remediate comprehensive repository review ([#5](https://github.com/jmagar/apprise-rmcp/issues/5)) ([a8f31ef](https://github.com/jmagar/apprise-rmcp/commit/a8f31efa65309e671b6e7d521f4d6ab52e33d6eb))
* route rust builds through sccache wrapper ([f34fcd1](https://github.com/jmagar/apprise-rmcp/commit/f34fcd1fb6810ee83f3511b740affc6078fd7c0b))
* **security:** update cmov for AArch64 correctness ([f3adde2](https://github.com/jmagar/apprise-rmcp/commit/f3adde2af113491f1e46576ec5153f60fa681f6d))

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
