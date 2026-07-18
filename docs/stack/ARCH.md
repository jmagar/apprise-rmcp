# apprise-rmcp architecture

```text
MCP client / rapprise CLI
          |
          +-- stdio (local process trust)
          +-- HTTP :40050 (bearer/OAuth outside loopback)
          |
     AppriseService
          |
     AppriseClient
          |
 external Apprise API (default http://localhost:8000)
          |
 configured notification backends
```

| Layer | Responsibility |
|---|---|
| `src/main.rs` | Process mode, config, HTTP auth assembly |
| `src/cli.rs` | CLI/output plus setup, doctor, and self-install |
| `src/mcp/` | Tool/prompt schema, dispatch, HTTP transport |
| `src/app.rs` | Notification application logic |
| `src/apprise.rs` | Outbound Apprise HTTP protocol |
| Apprise API | Destinations, tags, credentials, delivery semantics |

`APPRISE_TOKEN` is outbound; `APPRISE_MCP_TOKEN` authenticates HTTP clients.
`notify_url` carries sensitive destination URLs by design. Unauthenticated
HTTP is loopback-only.
