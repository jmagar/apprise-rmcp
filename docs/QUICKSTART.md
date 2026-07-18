# apprise-rmcp quickstart

Prerequisites are an Apprise API server plus Rust 1.86+ for source builds or
Node.js for the npm launcher. `APPRISE_URL` is optional and defaults to
`http://localhost:8000`.

```bash
git clone https://github.com/jmagar/apprise-rmcp
cd apprise-rmcp
cargo build --release
./target/release/rapprise health --json
./target/release/rapprise notify "Deploy complete" --tag ops --type success
```

For another upstream, set `APPRISE_URL`; set `APPRISE_TOKEN` only if it is
protected.

## Stdio MCP

```json
{
  "mcpServers": {
    "apprise": {
      "command": "npx",
      "args": ["-y", "apprise-rmcp", "mcp"],
      "env": { "APPRISE_URL": "http://localhost:8000" }
    }
  }
}
```

Stdio uses the local process boundary and ignores HTTP MCP auth settings.

## HTTP MCP

```bash
APPRISE_MCP_HOST=127.0.0.1 APPRISE_MCP_NO_AUTH=true ./target/release/rapprise serve
curl -sf http://127.0.0.1:40050/health
```

Any non-loopback deployment requires bearer or OAuth auth and TLS. See the
[auth table](INVENTORY.md#authentication-decision-table).

## Claude plugin

The plugin is bundled stdio, not a service deployer:

```bash
just plugin-build
claude plugin install ./plugins/apprise
```

Its hook runs bundled `rapprise setup plugin-hook`. Configure upstream values
in process env or `${APPRISE_HOME:-~/.apprise}/.env`.
