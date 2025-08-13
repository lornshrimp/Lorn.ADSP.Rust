//! 组件宏集成测试

use component_macros::{component, configurable, lifecycle};
use infrastructure_common::{Component, Configurable, DependencyAware, Lifecycle};
use serde::{Deserialize, Serialize};

/// 测试服务配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestServiceConfig {
    pub enabled: bool,
    pub timeout: u64,
    pub max_connections: usize,
}

impl Default for TestServiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: 30,
            max_connections: 100,
        }
    }
}

/// 测试服务
#[derive(Debug)]
#[component(singleton, priority = 100)]
#[configurable(path = "services.test_service")]
#[lifecycle(
    on_start = "initialize",
    on_stop = "cleanup",
    depends_on = ["DatabaseService", "CacheService"]
)]
pub struct TestService {
    config: Option<TestServiceConfig>,
    initialized: bool,
}

impl TestService {
    pub fn new() -> Self {
        Self {
            config: None,
            initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.initialized = true;
        println!("TestService initialized");
        Ok(())
    }

    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.initialized = false;
        println!("TestService cleaned up");
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// 简单组件测试
#[derive(Debug)]
#[component(transient)]
pub struct SimpleComponent {
    name: String,
}

impl SimpleComponent {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

/// 可配置组件测试
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimpleConfig {
    pub value: i32,
}

impl Default for SimpleConfig {
    fn default() -> Self {
        Self { value: 42 }
    }
}

#[derive(Debug)]
#[configurable(path = "components.simple", optional, default)]
pub struct ConfigurableComponent {
    config: Option<SimpleConfig>,
}

impl ConfigurableComponent {
    pub fn new() -> Self {
        Self { config: None }
    }

    pub fn get_config_value(&self) -> Option<i32> {
        self.config.as_ref().map(|c| c.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_trait_implementation() {
        let service = TestService::new();

        // 测试 Component trait 实现
        assert_eq!(service.name(), "TestService");
        assert_eq!(service.priority(), 100);
        assert!(service.is_enabled());
    }

    #[test]
    fn test_configurable_trait_implementation() {
        let mut service = TestService::new();

        // 测试 Configurable trait 实现
        assert_eq!(TestService::get_config_path(), "services.test_service");

        let config = TestServiceConfig {
            enabled: true,
            timeout: 60,
            max_connections: 200,
        };

        // 配置应该能够成功应用
        assert!(service.configure(config).is_ok());
    }

    #[tokio::test]
    async fn test_lifecycle_trait_implementation() {
        let mut service = TestService::new();

        // 测试生命周期管理
        assert!(!service.is_initialized());

        // 启动服务
        assert!(service.on_start().await.is_ok());
        assert!(service.is_initialized());

        // 停止服务
        assert!(service.on_stop().await.is_ok());
        assert!(!service.is_initialized());
    }

    #[test]
    fn test_dependency_aware_trait_implementation() {
        let service = TestService::new();

        // 测试依赖感知
        let dependencies = service.get_dependencies();
        assert_eq!(dependencies.len(), 2);
        assert!(dependencies.contains(&"DatabaseService".to_string()));
        assert!(dependencies.contains(&"CacheService".to_string()));
        assert!(!service.can_start_without_dependencies());
    }

    #[test]
    fn test_simple_component() {
        let component = SimpleComponent::new("test".to_string());

        // 测试基本组件功能
        assert_eq!(component.name(), "SimpleComponent");
        assert_eq!(component.priority(), 0); // 默认优先级
        assert!(component.is_enabled());
        assert_eq!(component.get_name(), "test");
    }

    #[test]
    fn test_configurable_component() {
        let mut component = ConfigurableComponent::new();

        // 测试可配置组件
        assert_eq!(
            ConfigurableComponent::get_config_path(),
            "components.simple"
        );
        assert_eq!(component.get_config_value(), None);

        let config = SimpleConfig { value: 123 };
        assert!(component.configure(config).is_ok());

        // 注意：这个测试可能需要根据实际的 configure 实现来调整
        // 因为当前的实现是空的
    }

    #[test]
    fn test_default_config() {
        let default_config = TestServiceConfig::default();
        assert!(default_config.enabled);
        assert_eq!(default_config.timeout, 30);
        assert_eq!(default_config.max_connections, 100);
    }
}
