# apprise-rmcp agent guide

`CLAUDE.md` is canonical. `AGENTS.md` and `GEMINI.md` must symlink to it.

## Product contract

- Repository/npm package: `apprise-rmcp`
- Rust crate/service: `apprise-mcp`
- Executable: `rapprise`
- MCP HTTP port: `40050`
- Upstream default: `http://localhost:8000`
- One `apprise` tool: `notify`, `notify_url`, `health`, `status`, `help`
- Data: `${APPRISE_HOME:-~/.apprise}` on hosts, `/data` in containers

## Architecture

`src/apprise.rs` owns Apprise HTTP; `src/app.rs` owns notification logic;
`src/mcp/` owns MCP contracts and routing. Notification CLI parsing delegates
to `AppriseService`, while setup, doctor, self-install, filesystem operations,
and output formatting currently live in `src/cli.rs`. HTTP auth is assembled
in `src/main.rs` and `src/mcp/routes.rs`.

Use sibling `foo.rs` plus `foo/`, never `foo/mod.rs`.

## Build and checks

```bash
cargo check
cargo test
cargo build --release
npm --prefix packages/apprise-rmcp test
tests/docs-contract.sh
```

## Plugin contract

`plugins/apprise` is the only plugin source and is a bundled stdio plugin.
`.mcp.json` launches `${CLAUDE_PLUGIN_ROOT}/bin/rapprise mcp`; hooks launch
the same bundled binary with `setup plugin-hook`. Build it with
`just plugin-build` before installing from a checkout. `setup check` is
read-only, `setup repair` is idempotent, and `--no-repair` audits.

Manifests do not advertise deployment options. Configure env or the canonical
data-directory `.env`. Do not add Docker/systemd/service bootstrap to hooks or
track a second plugin under `.claude/plugins`.

## Auth invariants

Stdio trusts the local parent process. HTTP no-auth is loopback-only. Bearer mode
uses `APPRISE_MCP_TOKEN`; OAuth requires issuer/client/admin state and must not
accept the static token when `disable_static_token_with_oauth=true`.
`APPRISE_TOKEN` is a distinct outbound upstream credential.

## Release invariant

Crate, npm launcher, registry package, `server.json`, release manifest, tag,
and assets use one coupled version. Registry identity is
`ai.dinglebear/apprise-rmcp`. Release Please owns version changes.

Use `bd` for all tracking: run `bd prime`, claim before editing, and close
completed work.
