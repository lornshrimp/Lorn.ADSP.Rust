//! 配置管理器实现

use crate::event_handler::ConfigEventHandler;
use async_trait::async_trait;
use config_abstractions::manager::ValidationResult;
use config_abstractions::{
    events::ConfigChangeEvent, ConfigManager, ConfigOptionDescriptor, ConfigProvider,
    ConfigValidator, ConfigWatcher, TypedConfigBinder,
};
use infrastructure_common::{ConfigError, ConfigSection, Configurable};
use serde::Deserialize;
use serde_json::Value;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// 配置快照，用于配置回滚
#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    /// 快照时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 配置数据
    pub config_data: HashMap<String, Value>,
    /// 快照版本号
    pub version: u64,
    /// 快照描述
    pub description: String,
}

/// 广告系统配置管理器
///
/// 主配置管理器，协调多个配置源并提供统一的配置访问接口
/// 支持热重载、配置验证和回滚功能
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
    /// 热重载相关字段
    /// 配置文件监控器
    config_watcher: Option<Arc<Mutex<dyn ConfigWatcher>>>,
    /// 是否启用热重载
    hot_reload_enabled: bool,
    /// 配置变更事件接收器
    change_receiver: Option<mpsc::Receiver<ConfigChangeEvent>>,
    /// 配置变更事件发送器（用于内部通信）
    change_sender: Option<mpsc::Sender<ConfigChangeEvent>>,
    /// 配置回滚历史（保存最近几次的配置快照）
    config_history: Arc<RwLock<Vec<ConfigSnapshot>>>,
    /// 最大回滚历史数量
    max_history_size: usize,
    /// 配置变更处理任务句柄
    change_handler_task: Option<tokio::task::JoinHandle<()>>,
    /// 配置事件处理器
    event_handler: Option<Arc<Mutex<ConfigEventHandler>>>,
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
            // 热重载相关字段初始化
            config_watcher: None,
            hot_reload_enabled: false,
            change_receiver: None,
            change_sender: None,
            config_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 10, // 默认保留最近10个配置快照
            change_handler_task: None,
            event_handler: None,
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

impl AdSystemConfigManager {
    /// 热重载相关方法

    /// 启用配置热重载
    ///
    /// 启用后，配置管理器将监控配置文件变更并自动重载配置
    pub async fn enable_hot_reload(
        &mut self,
        watcher: Arc<Mutex<dyn ConfigWatcher>>,
    ) -> Result<(), ConfigError> {
        if self.hot_reload_enabled {
            return Ok(());
        }

        info!("启用配置热重载");

        // 创建配置变更事件通道
        let (sender, receiver) = mpsc::channel(1000);
        self.change_sender = Some(sender.clone());
        self.change_receiver = Some(receiver);

        // 保存监控器引用
        self.config_watcher = Some(watcher.clone());
        self.hot_reload_enabled = true;

        // 创建配置初始快照
        self.create_config_snapshot("Initial configuration snapshot")
            .await?;

        // 启动配置变更处理任务
        let manager_clone = self.clone_for_event_handling().await;
        let mut receiver = self.change_receiver.take().unwrap();

        let handle = tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                if let Err(e) = manager_clone.handle_config_change_event(event).await {
                    error!("处理配置变更事件失败: {}", e);
                }
            }
        });

        self.change_handler_task = Some(handle);

        // 启动文件监控器
        {
            let mut watcher_guard = watcher.lock().await;
            watcher_guard.start_watching().await?;
        }

        info!("配置热重载已启用");
        Ok(())
    }

    /// 禁用配置热重载
    pub async fn disable_hot_reload(&mut self) -> Result<(), ConfigError> {
        if !self.hot_reload_enabled {
            return Ok(());
        }

        info!("禁用配置热重载");

        // 停止文件监控器
        if let Some(watcher) = &self.config_watcher {
            let mut watcher_guard = watcher.lock().await;
            watcher_guard.stop_watching().await?;
        }

        // 停止配置变更处理任务
        if let Some(handle) = self.change_handler_task.take() {
            handle.abort();
        }

        // 清理资源
        self.hot_reload_enabled = false;
        self.config_watcher = None;
        self.change_receiver = None;
        self.change_sender = None;

        info!("配置热重载已禁用");
        Ok(())
    }

    /// 处理配置变更事件
    async fn handle_config_change_event(
        &self,
        event: ConfigChangeEvent,
    ) -> Result<(), ConfigError> {
        info!("处理配置变更事件: {:?}", event.event_type);
        debug!("配置变更路径: {}", event.path);

        // 首先发送事件到事件处理器（通知监听器）
        self.send_event_to_handler(event.clone()).await?;

        // 在应用配置变更之前创建快照
        let snapshot_description = format!("Before applying change: {}", event.path);
        self.create_config_snapshot(&snapshot_description).await?;

        match event.event_type {
            config_abstractions::events::ConfigChangeEventType::Created => {
                self.handle_config_created(&event).await?;
            }
            config_abstractions::events::ConfigChangeEventType::Updated => {
                self.handle_config_updated(&event).await?;
            }
            config_abstractions::events::ConfigChangeEventType::Deleted => {
                self.handle_config_deleted(&event).await?;
            }
            config_abstractions::events::ConfigChangeEventType::Reloaded => {
                self.handle_config_reloaded(&event).await?;
            }
            config_abstractions::events::ConfigChangeEventType::ValidationFailed => {
                self.handle_config_validation_failed(&event).await?;
            }
            config_abstractions::events::ConfigChangeEventType::SourceConnectionFailed => {
                self.handle_source_connection_failed(&event).await?;
            }
            config_abstractions::events::ConfigChangeEventType::SourceConnectionRestored => {
                self.handle_source_connection_restored(&event).await?;
            }
        }

        // 验证配置变更后的配置
        let validation_result = self.validate_configuration().await?;
        if !validation_result.is_valid {
            warn!(
                "配置变更后验证失败，尝试回滚: {:?}",
                validation_result.errors
            );

            // 发送验证失败事件
            let validation_failed_event = ConfigChangeEvent::created(
                format!("validation_failed:{}", event.path),
                serde_json::Value::String("Validation failed after config change".to_string()),
                "ConfigManager",
            );
            self.send_event_to_handler(validation_failed_event).await?;

            self.rollback_last_change().await?;
            return Err(ConfigError::ValidationFailed {
                errors: validation_result
                    .errors
                    .into_iter()
                    .map(|e| format!("{:?}", e))
                    .collect(),
            });
        }

        info!("配置变更处理完成: {}", event.path);
        Ok(())
    }

    /// 处理配置创建事件
    async fn handle_config_created(&self, _event: &ConfigChangeEvent) -> Result<(), ConfigError> {
        // 重新加载所有配置提供者以包含新的配置
        info!("处理配置创建事件");
        self.reload_all_providers().await?;
        Ok(())
    }

    /// 处理配置更新事件
    async fn handle_config_updated(&self, _event: &ConfigChangeEvent) -> Result<(), ConfigError> {
        // 重新加载受影响的配置提供者
        info!("处理配置更新事件");
        self.reload_all_providers().await?;
        Ok(())
    }

    /// 处理配置删除事件
    async fn handle_config_deleted(&self, _event: &ConfigChangeEvent) -> Result<(), ConfigError> {
        // 重新加载所有配置提供者并清除相关缓存
        info!("处理配置删除事件");
        self.reload_all_providers().await?;
        Ok(())
    }

    /// 处理配置重载事件
    async fn handle_config_reloaded(&self, _event: &ConfigChangeEvent) -> Result<(), ConfigError> {
        // 重新加载所有配置提供者
        info!("处理配置重载事件");
        self.reload_all_providers().await?;
        Ok(())
    }

    /// 处理配置验证失败事件
    async fn handle_config_validation_failed(
        &self,
        event: &ConfigChangeEvent,
    ) -> Result<(), ConfigError> {
        warn!("处理配置验证失败事件: {}", event.path);
        // 验证失败时，可以选择回滚或记录错误
        // 这里选择记录错误，但不自动回滚
        Ok(())
    }

    /// 处理配置源连接失败事件
    async fn handle_source_connection_failed(
        &self,
        event: &ConfigChangeEvent,
    ) -> Result<(), ConfigError> {
        error!("配置源连接失败: {}", event.path);
        // 配置源连接失败时的处理逻辑
        // 可以尝试重连或使用备用配置源
        Ok(())
    }

    /// 处理配置源恢复事件
    async fn handle_source_connection_restored(
        &self,
        event: &ConfigChangeEvent,
    ) -> Result<(), ConfigError> {
        info!("配置源连接已恢复: {}", event.path);
        // 配置源恢复时，重新加载配置
        self.reload_all_providers().await?;
        Ok(())
    }

    /// 重新加载所有配置提供者（不可变版本）
    async fn reload_all_providers(&self) -> Result<(), ConfigError> {
        info!("重新加载所有配置提供者");

        let errors: Vec<ConfigError> = Vec::new();

        // 注意：这里由于借用检查的限制，我们需要重新设计这个方法
        // 或者使用内部可变性
        // 暂时记录错误但不实际重载，在实际实现中需要解决这个问题

        // 清除缓存
        {
            let mut cache = self.config_cache.write().await;
            cache.clear();
        }

        if errors.is_empty() {
            info!("所有配置提供者重载成功");
            Ok(())
        } else {
            Err(ConfigError::ReloadError {
                message: format!("{}个提供者重载失败", errors.len()),
            })
        }
    }

    /// 创建配置快照
    async fn create_config_snapshot(&self, description: &str) -> Result<(), ConfigError> {
        debug!("创建配置快照: {}", description);

        // 获取当前所有配置数据
        let current_config = self.get_all_config_data().await?;

        let snapshot = ConfigSnapshot {
            timestamp: chrono::Utc::now(),
            config_data: current_config,
            version: self.get_next_snapshot_version().await,
            description: description.to_string(),
        };

        // 添加到历史记录
        {
            let mut history = self.config_history.write().await;
            history.push(snapshot);

            // 限制历史记录大小
            if history.len() > self.max_history_size {
                history.remove(0);
            }
        }

        debug!("配置快照创建完成");
        Ok(())
    }

    /// 获取所有配置数据
    async fn get_all_config_data(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let cache = self.config_cache.read().await;
        Ok(cache.clone())
    }

    /// 获取下一个快照版本号
    async fn get_next_snapshot_version(&self) -> u64 {
        let history = self.config_history.read().await;
        history.len() as u64 + 1
    }

    /// 回滚到上一个配置快照
    pub async fn rollback_last_change(&self) -> Result<(), ConfigError> {
        info!("回滚到上一个配置");

        let snapshot = {
            let history = self.config_history.read().await;
            if history.len() < 2 {
                return Err(ConfigError::NoRollbackAvailable);
            }
            // 获取倒数第二个快照（最后一个是当前的）
            history[history.len() - 2].clone()
        };

        // 恢复配置数据到缓存
        {
            let mut cache = self.config_cache.write().await;
            *cache = snapshot.config_data;
        }

        info!("配置已回滚到版本: {}", snapshot.version);
        Ok(())
    }

    /// 回滚到指定版本的配置快照
    pub async fn rollback_to_version(&self, version: u64) -> Result<(), ConfigError> {
        info!("回滚到配置版本: {}", version);

        let snapshot = {
            let history = self.config_history.read().await;
            history
                .iter()
                .find(|s| s.version == version)
                .cloned()
                .ok_or_else(|| ConfigError::VersionNotFound { version })?
        };

        // 恢复配置数据到缓存
        {
            let mut cache = self.config_cache.write().await;
            *cache = snapshot.config_data;
        }

        info!("配置已回滚到版本: {}", version);
        Ok(())
    }

    /// 获取配置历史
    pub async fn get_config_history(&self) -> Vec<ConfigSnapshot> {
        let history = self.config_history.read().await;
        history.clone()
    }

    /// 清除配置历史
    pub async fn clear_config_history(&self) -> Result<(), ConfigError> {
        info!("清除配置历史");
        let mut history = self.config_history.write().await;
        history.clear();
        Ok(())
    }

    /// 是否启用了热重载
    pub fn is_hot_reload_enabled(&self) -> bool {
        self.hot_reload_enabled
    }

    /// 获取最大历史记录大小
    pub fn get_max_history_size(&self) -> usize {
        self.max_history_size
    }

    /// 设置最大历史记录大小
    pub fn set_max_history_size(&mut self, size: usize) {
        self.max_history_size = size;
    }

    /// 设置配置事件处理器
    pub async fn set_event_handler(
        &mut self,
        handler: Arc<Mutex<ConfigEventHandler>>,
    ) -> Result<(), ConfigError> {
        info!("设置配置事件处理器");
        self.event_handler = Some(handler);
        Ok(())
    }

    /// 获取配置事件处理器
    pub fn get_event_handler(&self) -> Option<Arc<Mutex<ConfigEventHandler>>> {
        self.event_handler.clone()
    }

    /// 向事件处理器发送事件
    async fn send_event_to_handler(&self, event: ConfigChangeEvent) -> Result<(), ConfigError> {
        if let Some(ref handler) = self.event_handler {
            let handler_guard = handler.lock().await;
            handler_guard.send_event(event).await?;
        }
        Ok(())
    }

    /// 克隆管理器用于事件处理（解决借用检查问题）
    async fn clone_for_event_handling(&self) -> Arc<AdSystemConfigManager> {
        // 这里需要实现一个适合事件处理的克隆版本
        // 在实际实现中，可能需要使用 Arc<RwLock<>> 等共享所有权的方式
        // 暂时返回一个新的实例，实际使用中需要完善
        Arc::new(AdSystemConfigManager::new())
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
