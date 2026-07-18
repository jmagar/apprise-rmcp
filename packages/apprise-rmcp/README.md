# apprise-rmcp

Node launcher for the `apprise-rmcp` Rust MCP server and `rapprise` CLI binary.

```bash
npx -y apprise-rmcp --help
npx -y apprise-rmcp health --json
npx -y apprise-rmcp mcp
```

The package downloads the matching `rapprise` release archive during
`postinstall`, fetches its published `.sha256`, rejects malformed or mismatched
digests, validates the single-file archive layout, and atomically installs the
binary. Release archives also carry GitHub build-provenance attestations.

The npm package, Rust binary, MCP registry metadata, and GitHub release use one
coupled version. A package at `0.1.3` downloads binary release `v0.1.3` unless an
explicit test/mirror override is supplied.

Supported prebuilt targets are Linux x86_64 and Windows x86_64. macOS, ARM, and
other targets must currently build the Rust crate from source.

The package exposes two npm binary aliases:

| Alias | Runs |
|---|---|
| `apprise-rmcp` | bundled `rapprise` binary |
| `rapprise` | bundled `rapprise` binary |

## MCP Client Example

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

## Launcher Controls

| Variable | Purpose |
|---|---|
| `APPRISE_RMCP_SKIP_DOWNLOAD=1` | Skip binary download during `postinstall`. |
| `APPRISE_RMCP_VERSION` or `APPRISE_RMCP_BINARY_VERSION` | Override the release tag used for downloads. |
| `APPRISE_RMCP_REPO` | Override the GitHub repo used for downloads. |
| `APPRISE_RMCP_RELEASE_BASE_URL` | Override the release download base URL. |
| `APPRISE_RMCP_DOWNLOAD_TIMEOUT_MS` | Bound the complete archive/checksum download (default 120000). |
| `APPRISE_RMCP_CONNECT_TIMEOUT_MS` | Bound connection establishment (default 10000). |
| `APPRISE_RMCP_MAX_REDIRECTS` | Maximum redirects per download (default 5). |

Full documentation lives in the repository README:
https://github.com/jmagar/apprise-rmcp#readme
