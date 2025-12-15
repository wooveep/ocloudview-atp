//! 协议类型和抽象测试

use atp_protocol::*;

#[test]
fn test_protocol_type_equality() {
    assert_eq!(ProtocolType::QMP, ProtocolType::QMP);
    assert_eq!(ProtocolType::QGA, ProtocolType::QGA);
    assert_eq!(ProtocolType::Spice, ProtocolType::Spice);

    assert_ne!(ProtocolType::QMP, ProtocolType::QGA);
    assert_ne!(ProtocolType::QGA, ProtocolType::Spice);
}

#[test]
fn test_protocol_type_virtio_serial() {
    let proto1 = ProtocolType::VirtioSerial("custom".to_string());
    let proto2 = ProtocolType::VirtioSerial("custom".to_string());
    let proto3 = ProtocolType::VirtioSerial("other".to_string());

    assert_eq!(proto1, proto2);
    assert_ne!(proto1, proto3);
}

#[test]
fn test_protocol_type_clone() {
    let original = ProtocolType::QMP;
    let cloned = original.clone();
    assert_eq!(original, cloned);

    let original_virtio = ProtocolType::VirtioSerial("test".to_string());
    let cloned_virtio = original_virtio.clone();
    assert_eq!(original_virtio, cloned_virtio);
}

#[test]
fn test_protocol_type_debug() {
    let qmp = ProtocolType::QMP;
    let debug_str = format!("{:?}", qmp);
    assert!(debug_str.contains("QMP"));

    let virtio = ProtocolType::VirtioSerial("test".to_string());
    let debug_str = format!("{:?}", virtio);
    assert!(debug_str.contains("VirtioSerial"));
    assert!(debug_str.contains("test"));
}

#[test]
fn test_protocol_error_variants() {
    let err1 = ProtocolError::ProtocolNotFound("test".to_string());
    assert!(matches!(err1, ProtocolError::ProtocolNotFound(_)));

    let err2 = ProtocolError::ProtocolAlreadyRegistered("test".to_string());
    assert!(matches!(err2, ProtocolError::ProtocolAlreadyRegistered(_)));

    let err3 = ProtocolError::ConnectionFailed("test".to_string());
    assert!(matches!(err3, ProtocolError::ConnectionFailed(_)));

    let err4 = ProtocolError::SendFailed("test".to_string());
    assert!(matches!(err4, ProtocolError::SendFailed(_)));

    let err5 = ProtocolError::ReceiveFailed("test".to_string());
    assert!(matches!(err5, ProtocolError::ReceiveFailed(_)));

    let err6 = ProtocolError::ParseError("test".to_string());
    assert!(matches!(err6, ProtocolError::ParseError(_)));

    let err7 = ProtocolError::CommandFailed("test".to_string());
    assert!(matches!(err7, ProtocolError::CommandFailed(_)));

    let err8 = ProtocolError::Timeout;
    assert!(matches!(err8, ProtocolError::Timeout));
}

#[test]
fn test_protocol_error_display() {
    let err = ProtocolError::ConnectionFailed("connection refused".to_string());
    let err_str = format!("{}", err);
    assert!(err_str.contains("协议连接失败"));
    assert!(err_str.contains("connection refused"));

    let err = ProtocolError::ProtocolNotFound("missing-protocol".to_string());
    let err_str = format!("{}", err);
    assert!(err_str.contains("不存在"));
    assert!(err_str.contains("missing-protocol"));

    let err = ProtocolError::Timeout;
    let err_str = format!("{}", err);
    assert!(err_str.contains("超时"));
}

#[tokio::test]
async fn test_protocol_registry() {
    let registry = ProtocolRegistry::new();

    // 测试空注册表 - get 返回 Result，所以应该检查 is_err
    assert!(registry.get("QMP").await.is_err());
    assert!(registry.get("QGA").await.is_err());
}
