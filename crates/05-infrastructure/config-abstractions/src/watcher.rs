//! 配置监控抽象接口

use async_trait::async_trait;
use infrastructure_common::ConfigError;
use crate::events::ConfigChangeEvent;
use std::path::Path;
use tokio::sync::mpsc;

/// 配置监控器 trait
/// 
/// 监控配置变更并发送事件通知
#[async_trait]
pub trait ConfigWatcher: Send + Sync {
    /// 开始监控
    async fn start_watching(&mut self) -> Result<(), ConfigError>;
    
    /// 停止监控
    async fn stop_watching(&mut self) -> Result<(), ConfigError>;
    
    /// 添加监控路径
    async fn add_watch_path(&mut self, path: &Path) -> Result<(), ConfigError>;
    
    /// 移除监控路径
    async fn remove_watch_path(&mut self, path: &Path) -> Result<(), ConfigError>;
    
    /// 获取变更事件接收器
    fn get_change_receiver(&self) -> mpsc::Receiver<ConfigChangeEvent>;
    
    /// 是否正在监控
    fn is_watching(&self) -> bool;
    
    /// 获取监控路径列表
    fn get_watched_paths(&self) -> Vec<std::path::PathBuf>;
}

/// 文件系统配置监控器 trait
#[async_trait]
pub trait FileSystemConfigWatcher: ConfigWatcher {
    /// 设置监控间隔
    fn set_watch_interval(&mut self, interval: std::time::Duration);
    
    /// 获取监控间隔
    fn get_watch_interval(&self) -> std::time::Duration;
    
    /// 设置防抖延迟
    fn set_debounce_delay(&mut self, delay: std::time::Duration);
    
    /// 获取防抖延迟
    fn get_debounce_delay(&self) -> std::time::Duration;
    
    /// 设置文件过滤器
    fn set_file_filter(&mut self, filter: Box<dyn FileFilter>);
}

/// 文件过滤器 trait
pub trait FileFilter: Send + Sync {
    /// 检查文件是否应该被监控
    fn should_watch(&self, path: &Path) -> bool;
    
    /// 获取过滤器名称
    fn name(&self) -> &str;
}

/// 扩展名文件过滤器
#[derive(Debug)]
pub struct ExtensionFileFilter {
    extensions: Vec<String>,
}

impl ExtensionFileFilter {
    /// 创建新的扩展名过滤器
    pub fn new(extensions: Vec<String>) -> Self {
        Self { extensions }
    }
    
    /// 创建常用配置文件过滤器
    pub fn config_files() -> Self {
        Self {
            extensions: vec![
                "toml".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "ini".to_string(),
                "cfg".to_string(),
            ],
        }
    }
}

impl FileFilter for ExtensionFileFilter {
    fn should_watch(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            self.extensions.iter().any(|ext| ext.eq_ignore_ascii_case(extension))
        } else {
            false
        }
    }
    
    fn name(&self) -> &str {
        "ExtensionFileFilter"
    }
}

/// 模式文件过滤器
#[derive(Debug)]
pub struct PatternFileFilter {
    patterns: Vec<glob::Pattern>,
}

impl PatternFileFilter {
    /// 创建新的模式过滤器
    pub fn new(patterns: Vec<String>) -> Result<Self, glob::PatternError> {
        let mut compiled_patterns = Vec::new();
        for pattern in patterns {
            compiled_patterns.push(glob::Pattern::new(&pattern)?);
        }
        Ok(Self {
            patterns: compiled_patterns,
        })
    }
}

impl FileFilter for PatternFileFilter {
    fn should_watch(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.patterns.iter().any(|pattern| pattern.matches(&path_str))
    }
    
    fn name(&self) -> &str {
        "PatternFileFilter"
    }
}
