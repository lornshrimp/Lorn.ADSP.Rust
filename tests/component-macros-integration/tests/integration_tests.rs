//! Centralized integration tests for component-macros crate (migrated)

use component_macros::{component, configurable, lifecycle};
use infrastructure_common::{Component, Configurable, Lifecycle}; // removed DependencyAware
use serde::{Deserialize, Serialize};

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
        Ok(())
    }
    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.initialized = false;
        Ok(())
    }
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

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

#[test]
fn test_component_trait_implementation() {
    let service = TestService::new();
    assert_eq!(service.name(), "TestService");
    assert_eq!(service.priority(), 100);
    assert!(service.is_enabled());
}

#[test]
fn test_configurable_trait_implementation() {
    let mut service = TestService::new();
    assert_eq!(TestService::get_config_path(), "services.test_service");
    let config = TestServiceConfig {
        enabled: true,
        timeout: 60,
        max_connections: 200,
    };
    assert!(service.configure(config).is_ok());
}

#[tokio::test]
async fn test_lifecycle_trait_implementation() {
    let mut service = TestService::new();
    assert!(!service.is_initialized());
    assert!(service.on_start().await.is_ok());
    assert!(service.is_initialized());
    assert!(service.on_stop().await.is_ok());
    assert!(!service.is_initialized());
}

#[test]
fn test_configurable_component() {
    let mut component = ConfigurableComponent::new();
    assert_eq!(
        ConfigurableComponent::get_config_path(),
        "components.simple"
    );
    assert_eq!(component.get_config_value(), None);
    let config = SimpleConfig { value: 123 };
    assert!(component.configure(config).is_ok());
}

#[test]
fn test_default_config() {
    let default_config = TestServiceConfig::default();
    assert!(default_config.enabled);
    assert_eq!(default_config.timeout, 30);
    assert_eq!(default_config.max_connections, 100);
}
