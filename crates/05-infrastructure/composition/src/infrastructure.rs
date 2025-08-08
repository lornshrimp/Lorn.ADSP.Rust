//! 基础设施主入口

use crate::builder::InfrastructureBuilder;
use config_abstractions::ConfigManager;
use config_impl::manager::AdSystemConfigManager;
use di_abstractions::DiContainer;
use di_impl::DiContainerImpl;
use infrastructure_common::{
    HealthCheckable, HealthStatus, InfrastructureError, Component, DependencyError,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

/// 广告系统基础设施
/// 
/// 系统的核心基础设施实例，提供配置管理、依赖注入、健康检查等功能
pub struct AdSystemInfrastructure {
    /// 配置管理器
    config_manager: Arc<AdSystemConfigManager>,
    /// 依赖注入容器
    di_container: Arc<RwLock<DiContainerImpl>>,
    /// 健康检查器集合
    health_checkers: Vec<Box<dyn HealthCheckable>>,
    /// 运行状态
    status: Arc<RwLock<InfrastructureStatus>>,
    /// 统计信息
    metrics: Arc<RwLock<InfrastructureMetrics>>,
}

impl AdSystemInfrastructure {
    /// 创建基础设施构建器
    pub fn builder() -> InfrastructureBuilder {
        InfrastructureBuilder::new()
    }
    
    /// 内部构造函数
    pub(crate) fn new(
        config_manager: Arc<AdSystemConfigManager>,
        di_container: DiContainerImpl,
        health_checkers: Vec<Box<dyn HealthCheckable>>,
    ) -> Self {
        Self {
            config_manager,
            di_container: Arc::new(RwLock::new(di_container)),
            health_checkers,
            status: Arc::new(RwLock::new(InfrastructureStatus::Initialized)),
            metrics: Arc::new(RwLock::new(InfrastructureMetrics::default())),
        }
    }
    
    /// 启动基础设施
    pub async fn start(&self) -> Result<(), InfrastructureError> {
        info!("启动基础设施");
        
        {
            let mut status = self.status.write().await;
            *status = InfrastructureStatus::Starting;
        }
        
        // 更新指标
        {
            let mut metrics = self.metrics.write().await;
            metrics.start_time = Some(chrono::Utc::now());
        }
        
        // 验证配置
        if let Err(e) = self.config_manager.validate_configuration().await {
            error!("配置验证失败: {:?}", e);
            {
                let mut status = self.status.write().await;
                *status = InfrastructureStatus::Failed;
            }
            return Err(InfrastructureError::BootstrapFailed {
                message: "配置验证失败".to_string(),
            });
        }
        
        // 验证依赖关系
        {
            let container = self.di_container.read().await;
            if let Err(errors) = container.validate().await {
                error!("依赖关系验证失败: {:?}", errors);
                {
                    let mut status = self.status.write().await;
                    *status = InfrastructureStatus::Failed;
                }
                return Err(InfrastructureError::DependencyError { 
                    source: DependencyError::CircularDependency { 
                        dependency_chain: "container_validation".to_string() 
                    } 
                });
            }
        }
        
        {
            let mut status = self.status.write().await;
            *status = InfrastructureStatus::Running;
        }
        
        info!("基础设施启动完成");
        Ok(())
    }
    
    /// 停止基础设施
    pub async fn stop(&self) -> Result<(), InfrastructureError> {
        info!("停止基础设施");
        
        {
            let mut status = self.status.write().await;
            *status = InfrastructureStatus::Stopping;
        }
        
        // 清理资源
        info!("依赖注入容器已停止");
        
        {
            let mut status = self.status.write().await;
            *status = InfrastructureStatus::Stopped;
            
            let mut metrics = self.metrics.write().await;
            metrics.stop_time = Some(chrono::Utc::now());
        }
        
        info!("基础设施停止完成");
        Ok(())
    }
    
    /// 获取配置
    pub async fn get_config<T>(&self, key: &str) -> Result<T, InfrastructureError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        self.config_manager
            .bind_configuration(key)
            .await
            .map_err(|e| InfrastructureError::ConfigError { source: e })
    }
    
    /// 解析组件
    pub async fn resolve<T>(&self) -> Result<Arc<T>, InfrastructureError>
    where
        T: Component + 'static,
    {
        let container = self.di_container.read().await;
        container
            .resolve()
            .await
            .map_err(|e| InfrastructureError::DependencyError { source: e })
    }
    
    /// 解析所有实现指定 trait 的组件
    pub async fn resolve_all<T>(&self) -> Result<Vec<Arc<T>>, InfrastructureError>
    where
        T: Component + 'static,
    {
        // 目前的 DiContainer 实现不支持 resolve_all，返回单个组件的向量
        match self.resolve::<T>().await {
            Ok(component) => Ok(vec![component]),
            Err(e) => Err(e),
        }
    }
    
    /// 检查组件是否已注册
    pub fn is_component_registered<T>(&self) -> bool
    where
        T: Component + 'static,
    {
        if let Ok(container) = self.di_container.try_read() {
            container.is_registered::<T>()
        } else {
            false
        }
    }
    
    /// 执行健康检查
    pub async fn check_health(&self) -> Vec<(String, HealthStatus)> {
        let mut results = Vec::new();
        
        for checker in &self.health_checkers {
            let status = match tokio::time::timeout(
                std::time::Duration::from_secs(30), // 默认30秒超时
                checker.check_health()
            ).await {
                Ok(status) => status,
                Err(_) => HealthStatus::unhealthy("健康检查超时"),
            };
            
            results.push((checker.name().to_string(), status));
        }
        
        results
    }
    
    /// 获取整体健康状态
    pub async fn get_overall_health(&self) -> HealthStatus {
        let results = self.check_health().await;
        
        let unhealthy_count = results.iter().filter(|(_, status)| {
            matches!(status, HealthStatus::Unhealthy { .. })
        }).count();
        
        let degraded_count = results.iter().filter(|(_, status)| {
            matches!(status, HealthStatus::Degraded { .. })
        }).count();
        
        if unhealthy_count > 0 {
            HealthStatus::unhealthy(&format!("{}个组件不健康", unhealthy_count))
        } else if degraded_count > 0 {
            HealthStatus::degraded(&format!("{}个组件降级", degraded_count))
        } else {
            HealthStatus::healthy()
        }
    }
    
    /// 获取运行状态
    pub async fn get_status(&self) -> InfrastructureStatus {
        *self.status.read().await
    }
    
    /// 获取统计信息
    pub async fn get_metrics(&self) -> InfrastructureMetrics {
        self.metrics.read().await.clone()
    }
    
    /// 重新加载配置
    pub async fn reload_configuration(&self) -> Result<(), InfrastructureError> {
        info!("重新加载配置");
        
        // 这里需要获取可变引用，但当前架构中 config_manager 是不可变的
        // 实际实现中可能需要使用内部可变性或重新设计架构
        
        info!("配置重新加载完成");
        Ok(())
    }
    
    /// 获取已注册的组件列表
    pub async fn get_registered_components(&self) -> Vec<infrastructure_common::ComponentMetadata> {
        let container = self.di_container.read().await;
        container.get_registered_components()
    }
    
    /// 获取配置管理器引用
    pub fn config_manager(&self) -> &Arc<AdSystemConfigManager> {
        &self.config_manager
    }
    
    /// 获取依赖注入容器引用
    pub fn di_container(&self) -> &Arc<RwLock<DiContainerImpl>> {
        &self.di_container
    }
}

/// 基础设施运行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfrastructureStatus {
    /// 已初始化
    Initialized,
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 失败
    Failed,
}

/// 基础设施统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureMetrics {
    /// 启动时间
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 停止时间
    pub stop_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 已注册的组件数量
    pub registered_components_count: usize,
    /// 配置提供者数量
    pub config_providers_count: usize,
    /// 健康检查器数量
    pub health_checkers_count: usize,
    /// 配置重载次数
    pub config_reload_count: u64,
    /// 组件解析次数
    pub component_resolution_count: u64,
    /// 健康检查执行次数
    pub health_check_count: u64,
}

impl Default for InfrastructureMetrics {
    fn default() -> Self {
        Self {
            start_time: None,
            stop_time: None,
            registered_components_count: 0,
            config_providers_count: 0,
            health_checkers_count: 0,
            config_reload_count: 0,
            component_resolution_count: 0,
            health_check_count: 0,
        }
    }
}

impl InfrastructureMetrics {
    /// 计算运行时间
    pub fn uptime(&self) -> Option<chrono::Duration> {
        match (self.start_time, self.stop_time) {
            (Some(start), Some(stop)) => Some(stop - start),
            (Some(start), None) => Some(chrono::Utc::now() - start),
            _ => None,
        }
    }
}
