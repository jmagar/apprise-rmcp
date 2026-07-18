# Prerequisites

Runtime requires a reachable Apprise API (default `http://localhost:8000`).
Network HTTP MCP requires TLS termination plus bearer or OAuth auth.

Development requires Rust 1.90+, Cargo/rustfmt/Clippy, Node.js for launcher
tests, `jq` for JSON checks, and Docker only for container smoke tests.

No syslog receiver, rsyslog, log forwarding, or system SQLite install is needed.
