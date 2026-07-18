// Public notification/config surface tests. The actual CLI parser is tested in
// `src/cli.rs`, next to the parser implementation.

#[test]
fn notify_type_parsing() {
    use apprise_mcp::apprise::NotifyType;

    assert_eq!(NotifyType::from_str_opt("info"), Some(NotifyType::Info));
    assert_eq!(
        NotifyType::from_str_opt("success"),
        Some(NotifyType::Success)
    );
    assert_eq!(
        NotifyType::from_str_opt("warning"),
        Some(NotifyType::Warning)
    );
    assert_eq!(NotifyType::from_str_opt("warn"), Some(NotifyType::Warning));
    assert_eq!(
        NotifyType::from_str_opt("failure"),
        Some(NotifyType::Failure)
    );
    assert_eq!(NotifyType::from_str_opt("fail"), Some(NotifyType::Failure));
    assert_eq!(NotifyType::from_str_opt("error"), Some(NotifyType::Failure));
    assert_eq!(NotifyType::from_str_opt("unknown"), None);
}

#[test]
fn notify_type_as_str() {
    use apprise_mcp::apprise::NotifyType;

    assert_eq!(NotifyType::Info.as_str(), "info");
    assert_eq!(NotifyType::Success.as_str(), "success");
    assert_eq!(NotifyType::Warning.as_str(), "warning");
    assert_eq!(NotifyType::Failure.as_str(), "failure");
}

#[test]
fn notify_type_default_is_info() {
    use apprise_mcp::apprise::NotifyType;
    assert_eq!(NotifyType::default(), NotifyType::Info);
}

#[test]
fn config_defaults() {
    use apprise_mcp::config::{AppriseConfig, McpConfig};

    let mcp = McpConfig::default();
    assert_eq!(mcp.port, 40050);
    assert_eq!(mcp.host, "0.0.0.0");
    assert_eq!(mcp.server_name, "apprise-mcp");
    assert!(mcp.api_token.is_none());

    let apprise = AppriseConfig::default();
    assert!(!apprise.url.is_empty(), "default URL should be non-empty");
}

#[test]
fn config_bind_addr() {
    use apprise_mcp::config::McpConfig;
    let cfg = McpConfig::default();
    assert_eq!(cfg.bind_addr(), "0.0.0.0:40050");
}
