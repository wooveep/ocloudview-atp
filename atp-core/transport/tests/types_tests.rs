//! 传输层基础类型测试

use atp_transport::*;
use std::collections::HashMap;

#[test]
fn test_host_info_new() {
    let host = HostInfo::new("test-host-1", "192.168.1.100");

    assert_eq!(host.id, "test-host-1");
    assert_eq!(host.host, "192.168.1.100");
    assert_eq!(host.uri, "qemu+ssh://192.168.1.100:22/system");
    assert!(host.tags.is_empty());
    assert!(host.metadata.is_empty());
}

#[test]
fn test_host_info_with_uri() {
    let host = HostInfo::new("test-host-1", "192.168.1.100")
        .with_uri("qemu+tcp://192.168.1.100/system");

    assert_eq!(host.uri, "qemu+tcp://192.168.1.100/system");
}

#[test]
fn test_host_info_with_tags() {
    let tags = vec!["production".to_string(), "high-cpu".to_string()];
    let host = HostInfo::new("test-host-1", "192.168.1.100")
        .with_tags(tags.clone());

    assert_eq!(host.tags, tags);
    assert_eq!(host.tags.len(), 2);
}

#[test]
fn test_host_info_with_metadata() {
    let host = HostInfo::new("test-host-1", "192.168.1.100")
        .with_metadata("region", "us-west")
        .with_metadata("zone", "az-1");

    assert_eq!(host.metadata.get("region"), Some(&"us-west".to_string()));
    assert_eq!(host.metadata.get("zone"), Some(&"az-1".to_string()));
    assert_eq!(host.metadata.len(), 2);
}

#[test]
fn test_host_info_builder_pattern() {
    let host = HostInfo::new("complex-host", "10.0.0.1")
        .with_uri("qemu:///system")
        .with_tags(vec!["development".to_string()])
        .with_metadata("env", "dev")
        .with_metadata("owner", "team-a");

    assert_eq!(host.id, "complex-host");
    assert_eq!(host.host, "10.0.0.1");
    assert_eq!(host.uri, "qemu:///system");
    assert_eq!(host.tags.len(), 1);
    assert_eq!(host.metadata.len(), 2);
}

#[test]
fn test_connection_state_equality() {
    use atp_transport::ConnectionState;

    assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
    assert_eq!(ConnectionState::Connecting, ConnectionState::Connecting);
    assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
    assert_eq!(ConnectionState::Failed, ConnectionState::Failed);

    assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);
    assert_ne!(ConnectionState::Connecting, ConnectionState::Failed);
}

#[test]
fn test_transport_error_variants() {
    let err1 = TransportError::ConnectionFailed("test error".to_string());
    assert!(matches!(err1, TransportError::ConnectionFailed(_)));

    let err2 = TransportError::HostNotFound("host-1".to_string());
    assert!(matches!(err2, TransportError::HostNotFound(_)));

    let err3 = TransportError::DomainNotFound("vm-1".to_string());
    assert!(matches!(err3, TransportError::DomainNotFound(_)));

    let err4 = TransportError::PoolExhausted;
    assert!(matches!(err4, TransportError::PoolExhausted));

    let err5 = TransportError::Timeout;
    assert!(matches!(err5, TransportError::Timeout));

    let err6 = TransportError::Disconnected;
    assert!(matches!(err6, TransportError::Disconnected));
}

#[test]
fn test_transport_error_display() {
    let err = TransportError::ConnectionFailed("connection refused".to_string());
    let err_str = format!("{}", err);
    assert!(err_str.contains("连接失败"));
    assert!(err_str.contains("connection refused"));

    let err = TransportError::HostNotFound("missing-host".to_string());
    let err_str = format!("{}", err);
    assert!(err_str.contains("不存在"));
    assert!(err_str.contains("missing-host"));
}

#[test]
fn test_host_info_clone() {
    let original = HostInfo::new("test-host", "192.168.1.1")
        .with_tags(vec!["tag1".to_string()])
        .with_metadata("key", "value");

    let cloned = original.clone();

    assert_eq!(cloned.id, original.id);
    assert_eq!(cloned.host, original.host);
    assert_eq!(cloned.uri, original.uri);
    assert_eq!(cloned.tags, original.tags);
    assert_eq!(cloned.metadata, original.metadata);
}
