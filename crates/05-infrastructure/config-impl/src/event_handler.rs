//! 配置变更事件处理器实现

use async_trait::async_trait;
use config_abstractions::events::{ConfigChangeEvent, ConfigChangeEventType, ConfigEventListener};
use infrastructure_common::ConfigError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// 配置事件处理器
///
/// 负责管理和分发配置变更事件到各个监听器
pub struct ConfigEventHandler {
    /// 事件监听器映射
    listeners: Arc<RwLock<HashMap<String, Arc<dyn ConfigEventListener>>>>,
    /// 事件分发通道
    event_sender: mpsc::Sender<ConfigChangeEvent>,
    /// 事件接收器（用于内部处理）
    event_receiver: Option<mpsc::Receiver<ConfigChangeEvent>>,
    /// 是否正在运行
    is_running: bool,
    /// 事件处理任务句柄
    handler_task: Option<tokio::task::JoinHandle<()>>,
}

impl ConfigEventHandler {
    /// 创建新的配置事件处理器
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1000);

        Self {
            listeners: Arc::new(RwLock::new(HashMap::new())),
            event_sender: sender,
            event_receiver: Some(receiver),
            is_running: false,
            handler_task: None,
        }
    }

    /// 注册事件监听器
    pub async fn register_listener(
        &mut self,
        listener: Arc<dyn ConfigEventListener>,
    ) -> Result<(), ConfigError> {
        info!("注册配置事件监听器: {}", listener.name());

        let mut listeners = self.listeners.write().await;
        listeners.insert(listener.name().to_string(), listener);

        Ok(())
    }

    /// 移除事件监听器
    pub async fn unregister_listener(&mut self, listener_name: &str) -> Result<(), ConfigError> {
        info!("移除配置事件监听器: {}", listener_name);

        let mut listeners = self.listeners.write().await;
        if listeners.remove(listener_name).is_some() {
            Ok(())
        } else {
            Err(ConfigError::KeyNotFound {
                key: listener_name.to_string(),
            })
        }
    }

    /// 发送配置变更事件
    pub async fn send_event(&self, event: ConfigChangeEvent) -> Result<(), ConfigError> {
        debug!("发送配置变更事件: {:?}", event.event_type);

        self.event_sender
            .send(event)
            .await
            .map_err(|e| ConfigError::HotReloadError {
                message: format!("发送事件失败: {}", e),
            })?;

        Ok(())
    }

    /// 启动事件处理器
    pub async fn start(&mut self) -> Result<(), ConfigError> {
        if self.is_running {
            return Ok(());
        }

        info!("启动配置事件处理器");

        let listeners = self.listeners.clone();
        let mut receiver =
            self.event_receiver
                .take()
                .ok_or_else(|| ConfigError::HotReloadError {
                    message: "事件接收器不可用".to_string(),
                })?;

        let handle = tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                ConfigEventHandler::dispatch_event(&listeners, event).await;
            }
        });

        self.handler_task = Some(handle);
        self.is_running = true;

        info!("配置事件处理器已启动");
        Ok(())
    }

    /// 停止事件处理器
    pub async fn stop(&mut self) -> Result<(), ConfigError> {
        if !self.is_running {
            return Ok(());
        }

        info!("停止配置事件处理器");

        if let Some(handle) = self.handler_task.take() {
            handle.abort();
        }

        self.is_running = false;

        info!("配置事件处理器已停止");
        Ok(())
    }

    /// 分发事件到监听器
    async fn dispatch_event(
        listeners: &Arc<RwLock<HashMap<String, Arc<dyn ConfigEventListener>>>>,
        event: ConfigChangeEvent,
    ) {
        let listeners_guard = listeners.read().await;

        for (name, listener) in listeners_guard.iter() {
            if !listener.is_enabled() {
                continue;
            }

            // 检查监听器是否对此事件类型感兴趣
            let interested_types = listener.interested_event_types();
            if !interested_types.is_empty() && !interested_types.contains(&event.event_type) {
                continue;
            }

            debug!("向监听器 {} 分发事件: {:?}", name, event.event_type);

            // 分发事件到监听器
            listener.on_config_changed(&event);
        }
    }

    /// 获取监听器数量
    pub async fn get_listener_count(&self) -> usize {
        let listeners = self.listeners.read().await;
        listeners.len()
    }

    /// 获取所有监听器名称
    pub async fn get_listener_names(&self) -> Vec<String> {
        let listeners = self.listeners.read().await;
        listeners.keys().cloned().collect()
    }

    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

impl Default for ConfigEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 日志记录事件监听器
///
/// 将所有配置变更事件记录到日志中
pub struct LoggingConfigEventListener {
    name: String,
    enabled: bool,
    interested_events: Vec<ConfigChangeEventType>,
}

impl LoggingConfigEventListener {
    /// 创建新的日志记录监听器
    pub fn new() -> Self {
        Self {
            name: "LoggingConfigEventListener".to_string(),
            enabled: true,
            interested_events: vec![
                ConfigChangeEventType::Created,
                ConfigChangeEventType::Updated,
                ConfigChangeEventType::Deleted,
                ConfigChangeEventType::Reloaded,
                ConfigChangeEventType::ValidationFailed,
                ConfigChangeEventType::SourceConnectionFailed,
                ConfigChangeEventType::SourceConnectionRestored,
            ],
        }
    }

    /// 设置是否启用
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 设置感兴趣的事件类型
    pub fn set_interested_events(&mut self, events: Vec<ConfigChangeEventType>) {
        self.interested_events = events;
    }
}

impl Default for LoggingConfigEventListener {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigEventListener for LoggingConfigEventListener {
    fn on_config_changed(&self, event: &ConfigChangeEvent) {
        match event.event_type {
            ConfigChangeEventType::Created => {
                info!("配置创建: {} at {}", event.path, event.timestamp);
            }
            ConfigChangeEventType::Updated => {
                info!("配置更新: {} at {}", event.path, event.timestamp);
            }
            ConfigChangeEventType::Deleted => {
                warn!("配置删除: {} at {}", event.path, event.timestamp);
            }
            ConfigChangeEventType::Reloaded => {
                info!("配置重载: {} at {}", event.path, event.timestamp);
            }
            ConfigChangeEventType::ValidationFailed => {
                error!("配置验证失败: {} at {}", event.path, event.timestamp);
            }
            ConfigChangeEventType::SourceConnectionFailed => {
                error!("配置源连接失败: {} at {}", event.path, event.timestamp);
            }
            ConfigChangeEventType::SourceConnectionRestored => {
                info!("配置源连接恢复: {} at {}", event.path, event.timestamp);
            }
        }

        if !event.metadata.is_empty() {
            debug!("事件元数据: {:?}", event.metadata);
        }
    }

    fn on_file_system_event(&self, event: &config_abstractions::events::FileSystemEvent) {
        debug!(
            "文件系统事件: {:?} for {}",
            event.event_type,
            event.path.display()
        );
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn interested_event_types(&self) -> Vec<ConfigChangeEventType> {
        self.interested_events.clone()
    }
}

/// 配置验证事件监听器
///
/// 专门处理配置验证相关的事件
pub struct ValidationConfigEventListener {
    name: String,
    enabled: bool,
    validation_count: std::sync::atomic::AtomicU64,
    error_count: std::sync::atomic::AtomicU64,
}

impl ValidationConfigEventListener {
    /// 创建新的验证监听器
    pub fn new() -> Self {
        Self {
            name: "ValidationConfigEventListener".to_string(),
            enabled: true,
            validation_count: std::sync::atomic::AtomicU64::new(0),
            error_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 获取验证次数
    pub fn get_validation_count(&self) -> u64 {
        self.validation_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 获取错误次数
    pub fn get_error_count(&self) -> u64 {
        self.error_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 重置计数器
    pub fn reset_counters(&self) {
        self.validation_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.error_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for ValidationConfigEventListener {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigEventListener for ValidationConfigEventListener {
    fn on_config_changed(&self, event: &ConfigChangeEvent) {
        self.validation_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        match event.event_type {
            ConfigChangeEventType::ValidationFailed => {
                self.error_count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                error!(
                    "配置验证失败统计更新: {} (总错误数: {})",
                    event.path,
                    self.get_error_count()
                );
            }
            ConfigChangeEventType::Updated | ConfigChangeEventType::Reloaded => {
                debug!(
                    "配置验证成功统计更新: {} (总验证数: {})",
                    event.path,
                    self.get_validation_count()
                );
            }
            _ => {
                // 其他事件类型不计入验证统计
            }
        }
    }

    fn on_file_system_event(&self, _event: &config_abstractions::events::FileSystemEvent) {
        // 验证监听器不处理文件系统事件
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn interested_event_types(&self) -> Vec<ConfigChangeEventType> {
        vec![
            ConfigChangeEventType::Updated,
            ConfigChangeEventType::Reloaded,
            ConfigChangeEventType::ValidationFailed,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_event_handler_creation() {
        let handler = ConfigEventHandler::new();
        assert!(!handler.is_running());
        assert_eq!(handler.get_listener_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_listener() {
        let mut handler = ConfigEventHandler::new();
        let listener = Arc::new(LoggingConfigEventListener::new());

        let result = handler.register_listener(listener).await;
        assert!(result.is_ok());
        assert_eq!(handler.get_listener_count().await, 1);
    }

    #[tokio::test]
    async fn test_unregister_listener() {
        let mut handler = ConfigEventHandler::new();
        let listener = Arc::new(LoggingConfigEventListener::new());
        let listener_name = listener.name().to_string();

        handler.register_listener(listener).await.unwrap();
        assert_eq!(handler.get_listener_count().await, 1);

        let result = handler.unregister_listener(&listener_name).await;
        assert!(result.is_ok());
        assert_eq!(handler.get_listener_count().await, 0);
    }

    #[test]
    fn test_logging_listener_creation() {
        let listener = LoggingConfigEventListener::new();
        assert!(listener.is_enabled());
        assert_eq!(listener.name(), "LoggingConfigEventListener");
        assert_eq!(listener.interested_event_types().len(), 7);
    }

    #[test]
    fn test_validation_listener_counters() {
        let listener = ValidationConfigEventListener::new();
        assert_eq!(listener.get_validation_count(), 0);
        assert_eq!(listener.get_error_count(), 0);

        let event = ConfigChangeEvent::updated(
            "test.config",
            serde_json::Value::String("old".to_string()),
            serde_json::Value::String("new".to_string()),
            "test_source",
        );

        listener.on_config_changed(&event);
        assert_eq!(listener.get_validation_count(), 1);
        assert_eq!(listener.get_error_count(), 0);
    }
}
