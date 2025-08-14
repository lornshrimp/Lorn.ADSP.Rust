//! 配置热重载功能基础测试

use super::super::*;
use config_abstractions::ConfigWatcher;
use infrastructure_common::ConfigError;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 测试配置管理器热重载功能基本启用
#[tokio::test]
async fn test_config_manager_hot_reload_basic() {
    let mut config_manager = manager::AdSystemConfigManager::new();

    // 创建测试用的配置监控器
    let watcher = create_test_watcher().await;

    // 启用热重载
    let result = config_manager.enable_hot_reload(watcher).await;
    assert!(result.is_ok(), "启用热重载应该成功");

    // 验证热重载已启用
    assert!(config_manager.is_hot_reload_enabled());
}

/// 测试高级配置验证器基本功能
#[tokio::test]
async fn test_advanced_config_validator_basic() {
    let _validator = advanced_validator::AdvancedConfigValidator::new();
    // 基本创建测试，验证构造函数工作正常
}

/// 测试配置事件处理器基本功能
#[tokio::test]
async fn test_config_event_handler_basic() {
    let _event_handler = event_handler::ConfigEventHandler::new();
    // 基本创建测试，验证构造函数工作正常
}

/// 测试配置文件监控器创建
#[tokio::test]
async fn test_config_file_watcher_creation() {
    let result = watcher::ConfigFileWatcher::new();
    assert!(result.is_ok(), "配置文件监控器创建应该成功");

    let watcher = result.unwrap();
    assert!(!watcher.is_watching(), "新创建的监控器应该处于未监控状态");
}

/// 测试配置监控器的路径管理
#[tokio::test]
async fn test_watcher_path_management() {
    let mut watcher = watcher::ConfigFileWatcher::new().unwrap();

    let test_path = PathBuf::from("test_config.json");

    // 添加监控路径
    let result = watcher.add_watch_path(&test_path).await;
    assert!(result.is_ok(), "添加监控路径应该成功");

    // 移除监控路径
    let result = watcher.remove_watch_path(&test_path).await;
    assert!(result.is_ok(), "移除监控路径应该成功");
}

/// 测试热重载禁用功能
#[tokio::test]
async fn test_hot_reload_disable() {
    let mut config_manager = manager::AdSystemConfigManager::new();
    let watcher = create_test_watcher().await;

    // 启用热重载
    config_manager.enable_hot_reload(watcher).await.unwrap();
    assert!(config_manager.is_hot_reload_enabled());

    // 禁用热重载
    config_manager.disable_hot_reload().await.unwrap();
    assert!(!config_manager.is_hot_reload_enabled());
}

/// 测试配置历史记录功能
#[tokio::test]
async fn test_config_history() {
    let config_manager = manager::AdSystemConfigManager::new();

    // 获取配置历史
    let history = config_manager.get_config_history().await;
    assert!(history.is_empty(), "新创建的配置管理器应该没有历史记录");
}

/// 辅助函数：创建测试用的配置监控器
async fn create_test_watcher() -> Arc<Mutex<dyn ConfigWatcher>> {
    let watcher = watcher::ConfigFileWatcher::new().unwrap();
    Arc::new(Mutex::new(watcher))
}

/// 集成测试：验证所有组件能够协同工作
#[tokio::test]
async fn test_hot_reload_integration() {
    let mut config_manager = manager::AdSystemConfigManager::new();
    let watcher = create_test_watcher().await;

    // 1. 启用热重载
    config_manager.enable_hot_reload(watcher).await.unwrap();
    assert!(config_manager.is_hot_reload_enabled());

    // 2. 验证配置历史记录功能
    let history = config_manager.get_config_history().await;
    // 启用热重载后，会创建初始配置快照，所以历史不为空
    assert!(!history.is_empty(), "启用热重载后应该有初始配置快照");

    // 3. 禁用热重载
    config_manager.disable_hot_reload().await.unwrap();
    assert!(!config_manager.is_hot_reload_enabled());
}
