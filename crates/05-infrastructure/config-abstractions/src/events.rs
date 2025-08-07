//! 配置变更事件定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 配置变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// 事件类型
    pub event_type: ConfigChangeEventType,
    /// 变更路径
    pub path: String,
    /// 旧值
    pub old_value: Option<serde_json::Value>,
    /// 新值
    pub new_value: Option<serde_json::Value>,
    /// 事件时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 事件来源
    pub source: String,
    /// 额外元数据
    pub metadata: HashMap<String, String>,
}

impl ConfigChangeEvent {
    /// 创建配置创建事件
    pub fn created(
        path: impl Into<String>,
        value: serde_json::Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            event_type: ConfigChangeEventType::Created,
            path: path.into(),
            old_value: None,
            new_value: Some(value),
            timestamp: chrono::Utc::now(),
            source: source.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// 创建配置更新事件
    pub fn updated(
        path: impl Into<String>,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            event_type: ConfigChangeEventType::Updated,
            path: path.into(),
            old_value: Some(old_value),
            new_value: Some(new_value),
            timestamp: chrono::Utc::now(),
            source: source.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// 创建配置删除事件
    pub fn deleted(
        path: impl Into<String>,
        old_value: serde_json::Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            event_type: ConfigChangeEventType::Deleted,
            path: path.into(),
            old_value: Some(old_value),
            new_value: None,
            timestamp: chrono::Utc::now(),
            source: source.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// 创建配置重载事件
    pub fn reloaded(
        path: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            event_type: ConfigChangeEventType::Reloaded,
            path: path.into(),
            old_value: None,
            new_value: None,
            timestamp: chrono::Utc::now(),
            source: source.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// 配置变更事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConfigChangeEventType {
    /// 配置项创建
    Created,
    /// 配置项更新
    Updated,
    /// 配置项删除
    Deleted,
    /// 配置重载
    Reloaded,
    /// 配置验证失败
    ValidationFailed,
    /// 配置源连接失败
    SourceConnectionFailed,
    /// 配置源恢复
    SourceConnectionRestored,
}

/// 文件系统事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemEvent {
    /// 事件类型
    pub event_type: FileSystemEventType,
    /// 文件路径
    pub path: PathBuf,
    /// 事件时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 文件系统事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileSystemEventType {
    /// 文件创建
    Created,
    /// 文件修改
    Modified,
    /// 文件删除
    Deleted,
    /// 文件重命名
    Renamed,
    /// 权限变更
    PermissionChanged,
}

/// 配置事件监听器 trait
pub trait ConfigEventListener: Send + Sync {
    /// 处理配置变更事件
    fn on_config_changed(&self, event: &ConfigChangeEvent);
    
    /// 处理文件系统事件
    fn on_file_system_event(&self, event: &FileSystemEvent);
    
    /// 获取监听器名称
    fn name(&self) -> &str;
    
    /// 是否启用
    fn is_enabled(&self) -> bool {
        true
    }
    
    /// 获取感兴趣的事件类型
    fn interested_event_types(&self) -> Vec<ConfigChangeEventType>;
}
