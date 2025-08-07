//! 健康检查相关接口定义

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "data")]
pub enum HealthStatus {
    /// 健康状态
    Healthy,
    /// 降级状态
    Degraded { 
        message: String,
        details: Option<HashMap<String, String>>,
    },
    /// 不健康状态
    Unhealthy { 
        error: String,
        details: Option<HashMap<String, String>>,
    },
}

impl HealthStatus {
    /// 创建健康状态
    pub fn healthy() -> Self {
        Self::Healthy
    }
    
    /// 创建降级状态
    pub fn degraded(message: impl Into<String>) -> Self {
        Self::Degraded {
            message: message.into(),
            details: None,
        }
    }
    
    /// 创建降级状态（带详情）
    pub fn degraded_with_details(
        message: impl Into<String>,
        details: HashMap<String, String>,
    ) -> Self {
        Self::Degraded {
            message: message.into(),
            details: Some(details),
        }
    }
    
    /// 创建不健康状态
    pub fn unhealthy(error: impl Into<String>) -> Self {
        Self::Unhealthy {
            error: error.into(),
            details: None,
        }
    }
    
    /// 创建不健康状态（带详情）
    pub fn unhealthy_with_details(
        error: impl Into<String>,
        details: HashMap<String, String>,
    ) -> Self {
        Self::Unhealthy {
            error: error.into(),
            details: Some(details),
        }
    }
    
    /// 检查是否健康
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }
    
    /// 检查是否降级
    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded { .. })
    }
    
    /// 检查是否不健康
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, Self::Unhealthy { .. })
    }
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// 组件名称
    pub component_name: String,
    /// 健康状态
    pub status: HealthStatus,
    /// 检查耗时
    pub duration: Duration,
    /// 检查时间
    pub checked_at: chrono::DateTime<chrono::Utc>,
    /// 额外信息
    pub additional_info: HashMap<String, String>,
}

impl HealthCheckResult {
    /// 创建新的健康检查结果
    pub fn new(
        component_name: impl Into<String>,
        status: HealthStatus,
        duration: Duration,
    ) -> Self {
        Self {
            component_name: component_name.into(),
            status,
            duration,
            checked_at: chrono::Utc::now(),
            additional_info: HashMap::new(),
        }
    }
    
    /// 添加额外信息
    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_info.insert(key.into(), value.into());
        self
    }
}

/// 健康检查 trait
#[async_trait]
pub trait HealthCheckable: Send + Sync {
    /// 执行健康检查
    async fn check_health(&self) -> HealthStatus;
    
    /// 获取组件名称
    fn name(&self) -> &str;
    
    /// 获取检查超时时间
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
    
    /// 获取检查间隔
    fn check_interval(&self) -> Duration {
        Duration::from_secs(60)
    }
    
    /// 是否启用健康检查
    fn is_enabled(&self) -> bool {
        true
    }
}

/// 聚合健康检查器
pub struct AggregateHealthChecker {
    checkers: Vec<Box<dyn HealthCheckable>>,
}

impl std::fmt::Debug for AggregateHealthChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AggregateHealthChecker")
            .field("checkers", &format!("{} checkers", self.checkers.len()))
            .finish()
    }
}

impl AggregateHealthChecker {
    /// 创建新的聚合健康检查器
    pub fn new() -> Self {
        Self {
            checkers: Vec::new(),
        }
    }
    
    /// 添加健康检查器
    pub fn add_checker(&mut self, checker: Box<dyn HealthCheckable>) {
        self.checkers.push(checker);
    }
    
    /// 执行所有健康检查
    pub async fn check_all(&self) -> Vec<HealthCheckResult> {
        let mut results = Vec::new();
        
        for checker in &self.checkers {
            if !checker.is_enabled() {
                continue;
            }
            
            let start = std::time::Instant::now();
            
            let status = match tokio::time::timeout(checker.timeout(), checker.check_health()).await {
                Ok(status) => status,
                Err(_) => HealthStatus::unhealthy("健康检查超时"),
            };
            
            let duration = start.elapsed();
            
            let result = HealthCheckResult::new(checker.name(), status, duration);
            results.push(result);
        }
        
        results
    }
    
    /// 获取整体健康状态
    pub async fn get_overall_health(&self) -> HealthStatus {
        let results = self.check_all().await;
        
        let unhealthy_count = results.iter().filter(|r| r.status.is_unhealthy()).count();
        let degraded_count = results.iter().filter(|r| r.status.is_degraded()).count();
        
        if unhealthy_count > 0 {
            let details = results
                .into_iter()
                .filter(|r| r.status.is_unhealthy())
                .map(|r| (r.component_name, format!("{:?}", r.status)))
                .collect();
            
            HealthStatus::unhealthy_with_details(
                format!("{}个组件不健康", unhealthy_count),
                details,
            )
        } else if degraded_count > 0 {
            let details = results
                .into_iter()
                .filter(|r| r.status.is_degraded())
                .map(|r| (r.component_name, format!("{:?}", r.status)))
                .collect();
            
            HealthStatus::degraded_with_details(
                format!("{}个组件降级", degraded_count),
                details,
            )
        } else {
            HealthStatus::healthy()
        }
    }
}

impl Default for AggregateHealthChecker {
    fn default() -> Self {
        Self::new()
    }
}
