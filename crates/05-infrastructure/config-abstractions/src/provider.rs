//! 配置提供者抽象接口

use async_trait::async_trait;
use infrastructure_common::{ConfigError, ConfigSection};
use serde_json::Value;
use std::collections::HashMap;

/// 配置提供者 trait
/// 
/// 定义从不同数据源获取配置的统一接口
#[async_trait]
pub trait ConfigProvider: Send + Sync {
    /// 获取配置值
    async fn get_configuration(&self, key: &str) -> Result<Value, ConfigError>;
    
    /// 获取配置节
    async fn get_section(&self, section_name: &str) -> Result<ConfigSection, ConfigError>;
    
    /// 重新加载配置
    async fn reload(&mut self) -> Result<(), ConfigError>;
    
    /// 检查配置键是否存在
    async fn contains_key(&self, key: &str) -> Result<bool, ConfigError>;
    
    /// 获取所有配置键
    async fn get_all_keys(&self) -> Result<Vec<String>, ConfigError>;
    
    /// 获取提供者名称
    fn name(&self) -> &str;
    
    /// 获取提供者优先级
    fn priority(&self) -> i32 {
        0
    }
    
    /// 是否支持热重载
    fn supports_hot_reload(&self) -> bool {
        false
    }
}

/// 文件配置提供者 trait
#[async_trait]
pub trait FileConfigProvider: ConfigProvider {
    /// 获取文件路径
    fn file_path(&self) -> &str;
    
    /// 检查文件是否存在
    async fn file_exists(&self) -> bool;
    
    /// 获取文件最后修改时间
    async fn last_modified(&self) -> Result<std::time::SystemTime, ConfigError>;
}

/// 环境变量配置提供者 trait
#[async_trait]
pub trait EnvironmentConfigProvider: ConfigProvider {
    /// 获取环境变量前缀
    fn prefix(&self) -> &str;
    
    /// 获取分隔符
    fn separator(&self) -> &str;
    
    /// 获取所有匹配的环境变量
    async fn get_matching_env_vars(&self) -> Result<HashMap<String, String>, ConfigError>;
}

/// 数据库配置提供者 trait
#[async_trait]
pub trait DatabaseConfigProvider: ConfigProvider {
    /// 获取连接字符串
    fn connection_string(&self) -> &str;
    
    /// 获取表名
    fn table_name(&self) -> &str;
    
    /// 测试数据库连接
    async fn test_connection(&self) -> Result<(), ConfigError>;
    
    /// 保存配置到数据库
    async fn save_configuration(&self, key: &str, value: &Value) -> Result<(), ConfigError>;
}

/// 缓存配置提供者 trait
#[async_trait]
pub trait CacheConfigProvider: ConfigProvider {
    /// 缓存过期时间（秒）
    fn cache_ttl(&self) -> u64;
    
    /// 清除缓存
    async fn clear_cache(&mut self) -> Result<(), ConfigError>;
    
    /// 预热缓存
    async fn warm_cache(&mut self) -> Result<(), ConfigError>;
    
    /// 获取缓存统计信息
    async fn get_cache_stats(&self) -> Result<CacheStats, ConfigError>;
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存项数量
    pub size: usize,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl CacheStats {
    /// 计算命中率
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}
