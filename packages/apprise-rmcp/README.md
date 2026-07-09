# apprise-rmcp

Node launcher for the `rapprise` Rust MCP server and CLI binary.

```bash
npx -y apprise-rmcp --help
```

The package downloads the matching GitHub Release binary during `postinstall`.

## MCP stdio

Use the package directly as an MCP command:

```json
{
  "mcpServers": {
    "apprise-rmcp": {
      "command": "npx",
      "args": ["-y", "apprise-rmcp"]
    }
  }
}
```

## Environment

- `APPRISE_RMCP_BINARY_VERSION`: release tag/version to download, defaulting to this npm package version.
- `APPRISE_RMCP_VERSION`: alias for `APPRISE_RMCP_BINARY_VERSION`.
- `APPRISE_RMCP_REPO`: GitHub `owner/repo`, defaulting to `jmagar/apprise-rmcp`.
- `APPRISE_RMCP_RELEASE_BASE_URL`: full release download base URL.
- `APPRISE_RMCP_SKIP_DOWNLOAD=1`: skip postinstall download.
