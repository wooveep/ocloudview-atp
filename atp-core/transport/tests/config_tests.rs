//! 传输层配置测试

use atp_transport::*;
use std::time::Duration;

#[test]
fn test_default_pool_config() {
    let config = PoolConfig::default();

    assert_eq!(config.min_connections_per_host, 1);
    assert_eq!(config.max_connections_per_host, 10);
    assert_eq!(config.idle_timeout, 300);
    assert_eq!(config.selection_strategy, SelectionStrategy::RoundRobin);
}

#[test]
fn test_custom_pool_config() {
    let config = PoolConfig {
        min_connections_per_host: 2,
        max_connections_per_host: 20,
        idle_timeout: 600,
        selection_strategy: SelectionStrategy::LeastConnections,
    };

    assert_eq!(config.min_connections_per_host, 2);
    assert_eq!(config.max_connections_per_host, 20);
    assert_eq!(config.idle_timeout, 600);
    assert_eq!(config.selection_strategy, SelectionStrategy::LeastConnections);
}

#[test]
fn test_pool_config_idle_timeout() {
    let config = PoolConfig {
        idle_timeout: 300,
        ..Default::default()
    };

    assert_eq!(config.idle_timeout(), Duration::from_secs(300));
}

#[test]
fn test_default_reconnect_config() {
    let config = ReconnectConfig::default();

    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.initial_delay, 1);
    assert_eq!(config.max_delay, 60);
    assert_eq!(config.backoff_multiplier, 2.0);
}

#[test]
fn test_custom_reconnect_config() {
    let config = ReconnectConfig {
        max_attempts: 3,
        initial_delay: 2,
        max_delay: 30,
        backoff_multiplier: 1.5,
    };

    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.initial_delay, 2);
    assert_eq!(config.max_delay, 30);
    assert_eq!(config.backoff_multiplier, 1.5);
}

#[test]
fn test_reconnect_delay_calculation() {
    let config = ReconnectConfig {
        initial_delay: 1,
        max_delay: 60,
        backoff_multiplier: 2.0,
        ..Default::default()
    };

    // 第一次重试: 1 * 2^0 = 1秒
    assert_eq!(config.calculate_delay(0), Duration::from_secs(1));

    // 第二次重试: 1 * 2^1 = 2秒
    assert_eq!(config.calculate_delay(1), Duration::from_secs(2));

    // 第三次重试: 1 * 2^2 = 4秒
    assert_eq!(config.calculate_delay(2), Duration::from_secs(4));

    // 第四次重试: 1 * 2^3 = 8秒
    assert_eq!(config.calculate_delay(3), Duration::from_secs(8));

    // 第七次重试: 1 * 2^6 = 64秒，但限制为 max_delay = 60秒
    assert_eq!(config.calculate_delay(6), Duration::from_secs(60));
}

#[test]
fn test_reconnect_config_duration_methods() {
    let config = ReconnectConfig {
        initial_delay: 2,
        max_delay: 120,
        ..Default::default()
    };

    assert_eq!(config.initial_delay(), Duration::from_secs(2));
    assert_eq!(config.max_delay(), Duration::from_secs(120));
}

#[test]
fn test_default_transport_config() {
    let config = TransportConfig::default();

    assert_eq!(config.connect_timeout, 30);
    assert_eq!(config.heartbeat_interval, 60);
    assert_eq!(config.auto_reconnect, true);
    assert_eq!(config.pool.max_connections_per_host, 10);
    assert_eq!(config.reconnect.max_attempts, 5);
}

#[test]
fn test_transport_config_duration_methods() {
    let config = TransportConfig {
        connect_timeout: 45,
        heartbeat_interval: 90,
        ..Default::default()
    };

    assert_eq!(config.connect_timeout(), Duration::from_secs(45));
    assert_eq!(config.heartbeat_interval(), Duration::from_secs(90));
}

#[test]
fn test_selection_strategy_equality() {
    assert_eq!(SelectionStrategy::RoundRobin, SelectionStrategy::RoundRobin);
    assert_eq!(SelectionStrategy::LeastConnections, SelectionStrategy::LeastConnections);
    assert_eq!(SelectionStrategy::Random, SelectionStrategy::Random);

    assert_ne!(SelectionStrategy::RoundRobin, SelectionStrategy::Random);
}

#[test]
fn test_selection_strategy_default() {
    assert_eq!(SelectionStrategy::default(), SelectionStrategy::RoundRobin);
}

#[test]
fn test_config_serialization() {
    let config = TransportConfig::default();

    // 测试序列化为 JSON
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    assert!(!json.is_empty());

    // 测试反序列化
    let deserialized: TransportConfig = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.connect_timeout, config.connect_timeout);
    assert_eq!(deserialized.heartbeat_interval, config.heartbeat_interval);
}

#[test]
fn test_reconnect_with_different_multipliers() {
    let config = ReconnectConfig {
        initial_delay: 1,
        max_delay: 100,
        backoff_multiplier: 1.5,
        ..Default::default()
    };

    // 使用 1.5 倍增因子
    assert_eq!(config.calculate_delay(0), Duration::from_secs(1));
    assert_eq!(config.calculate_delay(1), Duration::from_secs(1)); // 1.5^1 = 1.5, 向下取整为 1
    assert_eq!(config.calculate_delay(2), Duration::from_secs(2)); // 1.5^2 = 2.25, 向下取整为 2
    assert_eq!(config.calculate_delay(3), Duration::from_secs(3)); // 1.5^3 = 3.375, 向下取整为 3
}
