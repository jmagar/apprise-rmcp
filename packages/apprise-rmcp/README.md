# apprise-rmcp

Node launcher for the `apprise-rmcp` Rust MCP server and `rapprise` CLI binary.

```bash
npx -y apprise-rmcp --help
npx -y apprise-rmcp health --json
npx -y apprise-rmcp mcp
```

The package downloads the matching `rapprise` release binary during
`postinstall`. It exposes two npm binary aliases:

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

Full documentation lives in the repository README:
https://github.com/jmagar/apprise-rmcp#readme
