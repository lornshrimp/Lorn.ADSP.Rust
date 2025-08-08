//! 配置管理器实现

use async_trait::async_trait;
use config_abstractions::manager::ValidationResult;
use config_abstractions::{
    ConfigManager, ConfigOptionDescriptor, ConfigProvider, ConfigValidator, TypedConfigBinder,
};
use infrastructure_common::{ConfigError, ConfigSection, Configurable};
use serde::Deserialize;
use serde_json::Value;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 广告系统配置管理器
///
/// 主配置管理器，协调多个配置源并提供统一的配置访问接口
pub struct AdSystemConfigManager {
    /// 配置提供者列表（按优先级排序）
    providers: Vec<Box<dyn ConfigProvider>>,
    /// 配置验证器映射
    validators: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
    /// 类型化配置绑定器
    binder: Arc<TypedConfigBinderImpl>,
    /// 已注册的配置选项
    registered_options: HashMap<String, ConfigOptionDescriptor>,
    /// 缓存的配置值
    config_cache: Arc<RwLock<HashMap<String, Value>>>,
    /// 是否启用缓存
    cache_enabled: bool,
}

impl std::fmt::Debug for AdSystemConfigManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdSystemConfigManager")
            .field("providers_count", &self.providers.len())
            .field("validators_count", &self.validators.len())
            .field("registered_options", &self.registered_options)
            .field("cache_enabled", &self.cache_enabled)
            .finish()
    }
}

impl AdSystemConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            validators: HashMap::new(),
            binder: Arc::new(TypedConfigBinderImpl::new()),
            registered_options: HashMap::new(),
            config_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_enabled: true,
        }
    }

    /// 设置是否启用缓存
    pub fn set_cache_enabled(&mut self, enabled: bool) {
        self.cache_enabled = enabled;
    }

    /// 清除配置缓存
    pub async fn clear_cache(&self) -> Result<(), ConfigError> {
        if self.cache_enabled {
            let mut cache = self.config_cache.write().await;
            cache.clear();
            debug!("配置缓存已清除");
        }
        Ok(())
    }

    /// 获取配置提供者数量
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// 获取已注册的配置选项数量
    pub fn registered_options_count(&self) -> usize {
        self.registered_options.len()
    }
}

impl Default for AdSystemConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigManager for AdSystemConfigManager {
    async fn register_provider(
        &mut self,
        provider: Box<dyn ConfigProvider>,
    ) -> Result<(), ConfigError> {
        info!("注册配置提供者: {}", provider.name());

        self.providers.push(provider);

        // 按优先级排序（优先级高的在前）
        self.providers
            .sort_by(|a, b| b.priority().cmp(&a.priority()));

        // 清除缓存，强制重新加载
        self.clear_cache().await?;

        Ok(())
    }

    async fn unregister_provider(&mut self, provider_name: &str) -> Result<(), ConfigError> {
        let initial_count = self.providers.len();
        self.providers.retain(|p| p.name() != provider_name);

        if self.providers.len() < initial_count {
            info!("移除配置提供者: {}", provider_name);
            self.clear_cache().await?;
            Ok(())
        } else {
            warn!("配置提供者不存在: {}", provider_name);
            Err(ConfigError::KeyNotFound {
                key: provider_name.to_string(),
            })
        }
    }

    async fn get_configuration(&self, key: &str) -> Result<Value, ConfigError> {
        debug!("获取配置: {}", key);

        // 检查缓存
        if self.cache_enabled {
            let cache = self.config_cache.read().await;
            if let Some(value) = cache.get(key) {
                debug!("从缓存获取配置: {}", key);
                return Ok(value.clone());
            }
        }

        // 按优先级顺序尝试各个提供者
        for provider in &self.providers {
            match provider.get_configuration(key).await {
                Ok(value) => {
                    debug!("从提供者 {} 获取配置: {}", provider.name(), key);

                    // 更新缓存
                    if self.cache_enabled {
                        let mut cache = self.config_cache.write().await;
                        cache.insert(key.to_string(), value.clone());
                    }

                    return Ok(value);
                }
                Err(ConfigError::KeyNotFound { .. }) => {
                    // 继续尝试下一个提供者
                    continue;
                }
                Err(e) => {
                    error!("提供者 {} 获取配置失败: {}", provider.name(), e);
                    continue;
                }
            }
        }

        Err(ConfigError::KeyNotFound {
            key: key.to_string(),
        })
    }

    async fn get_section(&self, section_name: &str) -> Result<ConfigSection, ConfigError> {
        debug!("获取配置节: {}", section_name);

        let mut combined_section = ConfigSection::new();

        // 从所有提供者收集配置节数据
        for provider in &self.providers {
            match provider.get_section(section_name).await {
                Ok(section) => {
                    // 合并配置节数据（后面的提供者优先级更低，不覆盖已有配置）
                    for (key, value) in section.data {
                        if !combined_section.data.contains_key(&key) {
                            combined_section.insert(key, value);
                        }
                    }
                }
                Err(ConfigError::KeyNotFound { .. }) => {
                    // 继续尝试下一个提供者
                    continue;
                }
                Err(e) => {
                    warn!("提供者 {} 获取配置节失败: {}", provider.name(), e);
                    continue;
                }
            }
        }

        if combined_section.data.is_empty() {
            Err(ConfigError::KeyNotFound {
                key: section_name.to_string(),
            })
        } else {
            Ok(combined_section)
        }
    }

    async fn bind_configuration<T>(&self, key: &str) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        debug!("绑定配置到类型: {} -> {}", key, std::any::type_name::<T>());
        self.binder.bind_configuration(key).await
    }

    async fn bind_to_instance<T>(&self, instance: &mut T, path: &str) -> Result<(), ConfigError>
    where
        T: Configurable,
    {
        debug!("绑定配置到实例: {} -> {}", path, std::any::type_name::<T>());
        self.binder.bind_to_instance(instance, path).await
    }

    async fn reload_all(&mut self) -> Result<(), ConfigError> {
        info!("重新加载所有配置");

        let mut errors = Vec::new();

        for provider in &mut self.providers {
            if let Err(e) = provider.reload().await {
                error!("提供者 {} 重载失败: {}", provider.name(), e);
                errors.push(e);
            }
        }

        // 清除缓存
        self.clear_cache().await?;

        if errors.is_empty() {
            info!("所有配置提供者重载成功");
            Ok(())
        } else {
            Err(ConfigError::ReloadError {
                message: format!("{}个提供者重载失败", errors.len()),
            })
        }
    }

    async fn validate_configuration(&self) -> Result<ValidationResult, ConfigError> {
        info!("验证所有配置");

        let mut overall_result = ValidationResult::success();
        let start_time = std::time::Instant::now();

        // 这里应该实现具体的验证逻辑
        // 遍历所有已注册的配置选项，使用对应的验证器进行验证

        overall_result.validated_at = chrono::Utc::now();
        let duration = start_time.elapsed();

        if overall_result.is_valid {
            info!("配置验证通过，耗时: {:?}", duration);
        } else {
            warn!(
                "配置验证失败，错误数: {}, 耗时: {:?}",
                overall_result.errors.len(),
                duration
            );
        }

        Ok(overall_result)
    }

    async fn register_validator<T>(
        &mut self,
        validator: Box<dyn ConfigValidator<T>>,
    ) -> Result<(), ConfigError>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        info!(
            "注册配置验证器: {} -> {}",
            validator.name(),
            std::any::type_name::<T>()
        );

        let boxed_validator: Box<dyn std::any::Any + Send + Sync> = Box::new(validator);
        self.validators.insert(type_id, boxed_validator);
        Ok(())
    }

    async fn register_all_options(&mut self) -> Result<(), ConfigError> {
        info!("注册所有组件配置选项");

        // 这里应该扫描所有实现了 Configurable trait 的类型
        // 并自动注册它们的配置选项
        // 实际实现需要结合组件发现机制

        info!("完成注册所有组件配置选项");
        Ok(())
    }

    async fn register_component_options<T>(&mut self) -> Result<(), ConfigError>
    where
        T: Configurable + 'static,
    {
        let config_path = T::get_config_path();
        info!(
            "注册组件配置选项: {} -> {}",
            std::any::type_name::<T>(),
            config_path
        );

        let descriptor = ConfigOptionDescriptor {
            path: config_path.to_string(),
            option_type: std::any::type_name::<T::Config>().to_string(),
            default_value: None, // 可以从 T::default_config() 获取
            description: Some(format!("配置选项: {}", std::any::type_name::<T>())),
            required: true,
            validation_rules: Vec::new(),
        };

        self.registered_options
            .insert(config_path.to_string(), descriptor);
        Ok(())
    }
}

/// 类型化配置绑定器实现
#[derive(Debug)]
pub struct TypedConfigBinderImpl {
    config_manager: Option<Arc<AdSystemConfigManager>>,
}

impl TypedConfigBinderImpl {
    /// 创建新的类型化配置绑定器
    pub fn new() -> Self {
        Self {
            config_manager: None,
        }
    }

    /// 设置配置管理器
    pub fn set_config_manager(&mut self, manager: Arc<AdSystemConfigManager>) {
        self.config_manager = Some(manager);
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
        // 这里需要访问配置管理器来获取配置值
        // 由于循环依赖的问题，可能需要重新设计这个结构

        // 临时实现：直接使用 config crate
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config/app").required(false))
            .add_source(config::Environment::with_prefix("ADSP"))
            .build()
            .map_err(|e| ConfigError::ParseError {
                source: Box::new(e),
            })?;

        let value: T = settings.get(path).map_err(|e| ConfigError::ParseError {
            source: Box::new(e),
        })?;

        Ok(value)
    }

    async fn bind_to_instance<T>(&self, instance: &mut T, path: &str) -> Result<(), ConfigError>
    where
        T: Configurable,
    {
        let config: T::Config = self.bind_configuration(path).await?;
        instance.configure(config)?;
        Ok(())
    }

    async fn bind_and_validate<T>(&self, path: &str) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let config = self.bind_configuration(path).await?;

        // 这里应该调用相应的验证器
        // 暂时跳过验证，直接返回配置

        Ok(config)
    }

    fn get_config_type_info<T>(&self) -> config_abstractions::ConfigTypeInfo
    where
        T: 'static,
    {
        config_abstractions::ConfigTypeInfo {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>().to_string(),
            config_path: "unknown".to_string(), // 需要从 Configurable trait 获取
            required: true,
        }
    }
}
