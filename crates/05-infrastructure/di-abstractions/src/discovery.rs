//! 组件发现抽象接口
//!
//! 提供组件自动发现和注册的能力

use infrastructure_common::{ComponentMetadata, ComponentError};
use async_trait::async_trait;
use std::collections::HashMap;

/// 组件发现器 trait
///
/// 用于自动发现和注册组件
#[async_trait]
pub trait ComponentDiscovery: Send + Sync {
    /// 发现组件
    async fn discover(&self, criteria: &DiscoveryCriteria) -> Result<Vec<ComponentMetadata>, ComponentError>;
    
    /// 获取发现器名称
    fn name(&self) -> &str;
    
    /// 检查是否支持指定的发现条件
    fn supports(&self, criteria: &DiscoveryCriteria) -> bool;
}

/// 发现条件
#[derive(Debug, Clone)]
pub struct DiscoveryCriteria {
    /// 搜索路径
    pub search_paths: Vec<String>,
    /// 包含的模式
    pub include_patterns: Vec<String>,
    /// 排除的模式
    pub exclude_patterns: Vec<String>,
    /// 额外的过滤条件
    pub filters: HashMap<String, String>,
    /// 是否递归搜索
    pub recursive: bool,
}

impl DiscoveryCriteria {
    /// 创建新的发现条件
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            filters: HashMap::new(),
            recursive: true,
        }
    }
    
    /// 添加搜索路径
    pub fn add_search_path<S: Into<String>>(mut self, path: S) -> Self {
        self.search_paths.push(path.into());
        self
    }
    
    /// 添加包含模式
    pub fn add_include_pattern<S: Into<String>>(mut self, pattern: S) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }
    
    /// 添加排除模式
    pub fn add_exclude_pattern<S: Into<String>>(mut self, pattern: S) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }
    
    /// 添加过滤条件
    pub fn add_filter<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.filters.insert(key.into(), value.into());
        self
    }
    
    /// 设置是否递归搜索
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }
}

impl Default for DiscoveryCriteria {
    fn default() -> Self {
        Self::new()
    }
}

/// 发现结果
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// 发现的组件元数据
    pub components: Vec<ComponentMetadata>,
    /// 发现器名称
    pub discoverer: String,
    /// 搜索路径
    pub search_path: String,
    /// 发现时间
    pub discovered_at: std::time::SystemTime,
}

impl DiscoveryResult {
    /// 创建新的发现结果
    pub fn new(components: Vec<ComponentMetadata>, discoverer: String, search_path: String) -> Self {
        Self {
            components,
            discoverer,
            search_path,
            discovered_at: std::time::SystemTime::now(),
        }
    }
}

/// 批量发现器
#[async_trait]
pub trait BatchDiscovery: Send + Sync {
    /// 批量发现组件
    async fn discover_batch(&self, criteria_list: &[DiscoveryCriteria]) -> Result<Vec<DiscoveryResult>, ComponentError>;
    
    /// 获取支持的发现条件数量限制
    fn max_batch_size(&self) -> usize {
        100
    }
}
