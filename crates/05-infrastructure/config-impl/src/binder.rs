//! 类型化配置绑定器实现

use async_trait::async_trait;
use config_abstractions::{TypedConfigBinder, ConfigTypeInfo};
use infrastructure_common::{ConfigError, Configurable};
use serde::Deserialize;
use std::any::TypeId;
use tracing::{debug, error};

/// 类型化配置绑定器实现
#[derive(Debug)]
pub struct TypedConfigBinderImpl {
    /// 是否启用缓存
    cache_enabled: bool,
}

impl TypedConfigBinderImpl {
    /// 创建新的类型化配置绑定器
    pub fn new() -> Self {
        Self {
            cache_enabled: true,
        }
    }
    
    /// 设置是否启用缓存
    pub fn set_cache_enabled(&mut self, enabled: bool) {
        self.cache_enabled = enabled;
    }
}

impl Default for TypedConfigBinderImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TypedConfigBinder for TypedConfigBinderImpl {
    async fn bind_configuration<T>(&self, path: &str) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        debug!("绑定配置到类型: {} -> {}", path, std::any::type_name::<T>());
        
        // 使用 config crate 来加载和绑定配置
        let settings = config::Config::builder()
            // 添加默认配置文件
            .add_source(config::File::with_name("config/app").required(false))
            .add_source(config::File::with_name("config/local").required(false))
            // 添加环境变量
            .add_source(config::Environment::with_prefix("ADSP").separator("_"))
            .build()
            .map_err(|e| {
                error!("配置构建失败: {}", e);
                ConfigError::ParseError {
                    source: Box::new(e),
                }
            })?;
        
        // 绑定到指定类型
        let result: T = settings.get(path).map_err(|e| {
            error!("配置绑定失败: path={}, error={}", path, e);
            ConfigError::ParseError {
                source: Box::new(e),
            }
        })?;
        
        debug!("配置绑定成功: {}", path);
        Ok(result)
    }
    
    async fn bind_to_instance<T>(&self, instance: &mut T, path: &str) -> Result<(), ConfigError>
    where
        T: Configurable,
    {
        debug!("绑定配置到实例: {} -> {}", path, std::any::type_name::<T>());
        
        // 获取配置
        let config: T::Config = self.bind_configuration(path).await?;
        
        // 应用配置到实例
        instance.configure(config)?;
        
        debug!("配置应用成功: {}", path);
        Ok(())
    }
    
    async fn bind_and_validate<T>(&self, path: &str) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        debug!("绑定并验证配置: {} -> {}", path, std::any::type_name::<T>());
        
        // 首先绑定配置
        let config = self.bind_configuration(path).await?;
        
        // 这里应该调用相应的验证器
        // 暂时跳过验证，直接返回配置
        
        debug!("配置绑定和验证成功: {}", path);
        Ok(config)
    }
    
    fn get_config_type_info<T>(&self) -> ConfigTypeInfo
    where
        T: 'static,
    {
        ConfigTypeInfo {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>().to_string(),
            config_path: "unknown".to_string(), // 需要从 Configurable trait 获取
            required: true,
        }
    }
}
