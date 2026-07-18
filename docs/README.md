# apprise-rmcp documentation

`apprise-rmcp` is a Rust MCP bridge and `rapprise` CLI for an external
Apprise API server.

| Document | Purpose |
|---|---|
| [Quickstart](QUICKSTART.md) | Build, configure, and call the server |
| [Inventory](INVENTORY.md) | Actions, config, auth, install support, versions |
| [Architecture](stack/ARCH.md) | Runtime layers and trust boundaries |
| [Technology](stack/TECH.md) | Implementation choices |
| [Prerequisites](stack/PRE-REQS.md) | Development/deployment requirements |
| [Rust guide](RUST.md) | Rust project notes |
| [Changelog](../CHANGELOG.md) | Release history |

Canonical package/repo name is `apprise-rmcp`, executable is `rapprise`,
MCP HTTP is port `40050`, and upstream defaults to
`http://localhost:8000`.
