//! 组件扫描器抽象接口
//!
//! 提供自动发现和扫描组件的能力

use infrastructure_common::{ComponentMetadata, ComponentError};
use async_trait::async_trait;

/// 组件扫描器 trait
///
/// 用于自动发现和扫描组件
#[async_trait]
pub trait ComponentScanner: Send + Sync {
    /// 扫描指定路径或模块中的组件
    async fn scan(&self, target: &str) -> Result<Vec<ComponentMetadata>, ComponentError>;
    
    /// 获取扫描器名称
    fn name(&self) -> &str;
    
    /// 检查是否支持指定的扫描目标
    fn supports(&self, target: &str) -> bool;
}

/// 扫描目标类型
#[derive(Debug, Clone)]
pub enum ScanTarget {
    /// 扫描指定的 crate
    Crate(String),
    /// 扫描指定的模块路径
    Module(String),
    /// 扫描指定的目录
    Directory(String),
    /// 扫描指定的文件
    File(String),
}

impl ScanTarget {
    /// 获取扫描目标的字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            ScanTarget::Crate(name) => name,
            ScanTarget::Module(path) => path,
            ScanTarget::Directory(path) => path,
            ScanTarget::File(path) => path,
        }
    }
}

/// 扫描选项
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// 是否递归扫描子目录
    pub recursive: bool,
    /// 包含的文件模式
    pub include_patterns: Vec<String>,
    /// 排除的文件模式
    pub exclude_patterns: Vec<String>,
    /// 是否扫描测试代码
    pub include_tests: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            include_patterns: vec!["*.rs".to_string()],
            exclude_patterns: vec!["target/*".to_string(), ".*".to_string()],
            include_tests: false,
        }
    }
}
