//! 配置相关的基础接口定义

use crate::errors::ConfigError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 可配置组件 trait
///
/// 需要配置的组件必须实现此 trait
pub trait Configurable: Send + Sync {
    /// 配置类型
    type Config: for<'de> Deserialize<'de> + Serialize + Clone + Send + Sync + 'static;

    /// 应用配置
    fn configure(&mut self, config: Self::Config) -> Result<(), ConfigError>;

    /// 获取配置路径
    fn get_config_path() -> &'static str;

    /// 获取默认配置
    fn default_config() -> Self::Config
    where
        Self::Config: Default,
    {
        Self::Config::default()
    }
}

/// 配置验证器 trait
pub trait ConfigValidator<T>: Send + Sync {
    /// 验证配置
    fn validate(&self, config: &T) -> Result<(), crate::errors::ValidationError>;

    /// 获取验证器名称
    fn name(&self) -> &'static str;
}

/// 配置节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSection {
    /// 配置数据
    pub data: HashMap<String, serde_json::Value>,
}

impl ConfigSection {
    /// 创建新的配置节
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// 插入配置项
    pub fn insert(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.data.insert(key.into(), value);
    }

    /// 获取配置项
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// 绑定到具体类型
    pub fn bind<T>(&self) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = serde_json::Value::Object(
            self.data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        serde_json::from_value(value).map_err(|e| ConfigError::SerializationError { source: e })
    }
}

impl Default for ConfigSection {
    fn default() -> Self {
        Self::new()
    }
}

/// 可配置组件基础实现
#[derive(Debug)]
pub struct ConfigurableComponent<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Clone + Send + Sync + 'static,
{
    pub config: Option<T>,
    pub config_path: &'static str,
}

impl<T> ConfigurableComponent<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Clone + Send + Sync + 'static,
{
    /// 创建新的可配置组件
    pub fn new(config_path: &'static str) -> Self {
        Self {
            config: None,
            config_path,
        }
    }

    /// 获取配置
    pub fn get_config(&self) -> Option<&T> {
        self.config.as_ref()
    }

    /// 设置配置
    pub fn set_config(&mut self, config: T) {
        self.config = Some(config);
    }
}

impl<T> Configurable for ConfigurableComponent<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Clone + Send + Sync + 'static,
{
    type Config = T;

    fn configure(&mut self, config: Self::Config) -> Result<(), ConfigError> {
        self.config = Some(config);
        Ok(())
    }

    fn get_config_path() -> &'static str {
        // 这里需要在具体实现中覆盖
        "unknown"
    }
}
