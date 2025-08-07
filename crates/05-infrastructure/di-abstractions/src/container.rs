//! 依赖注入容器抽象接口
//!
//! 提供依赖注入容器的核心抽象

use crate::registry::ComponentRegistry;
use crate::resolver::{ComponentResolver, ResolveContext};
use crate::factory::ComponentFactory;
use crate::scanner::ComponentScanner;
use crate::discovery::ComponentDiscovery;
use infrastructure_common::{Component, DependencyError, ComponentError, ComponentMetadata};
use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::sync::Arc;

/// 依赖注入容器 trait
///
/// 提供完整的依赖注入功能
#[async_trait]
pub trait DiContainer: Send + Sync {
    /// 注册组件
    async fn register<T>(&mut self, metadata: ComponentMetadata) -> Result<(), ComponentError>
    where
        T: Component + 'static;
    
    /// 注册单例组件
    async fn register_singleton<T>(&mut self, instance: Arc<T>) -> Result<(), ComponentError>
    where
        T: Component + 'static;
    
    /// 注册工厂
    async fn register_factory<T>(&mut self, factory: Box<dyn ComponentFactory>) -> Result<(), ComponentError>
    where
        T: Component + 'static;
    
    /// 解析组件
    async fn resolve<T>(&self) -> Result<Arc<T>, DependencyError>
    where
        T: Component + 'static;
    
    /// 解析组件（使用 TypeId）
    async fn resolve_by_type_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>, DependencyError>;
    
    /// 解析组件（使用名称）
    async fn resolve_by_name(&self, name: &str) -> Result<Arc<dyn Any + Send + Sync>, DependencyError>;
    
    /// 检查是否已注册组件
    fn is_registered<T>(&self) -> bool
    where
        T: Component + 'static;
    
    /// 检查是否已注册组件（使用 TypeId）
    fn is_registered_by_type_id(&self, type_id: TypeId) -> bool;
    
    /// 检查是否已注册组件（使用名称）
    fn is_registered_by_name(&self, name: &str) -> bool;
    
    /// 获取所有已注册的组件元数据
    fn get_registered_components(&self) -> Vec<ComponentMetadata>;
    
    /// 扫描并注册组件
    async fn scan_and_register(&mut self, target: &str) -> Result<usize, ComponentError>;
    
    /// 验证容器状态
    async fn validate(&self) -> Result<(), Vec<ComponentError>>;
}

/// 容器构建器 trait
pub trait ContainerBuilder: Send + Sync {
    /// 关联的容器类型
    type Container: DiContainer;
    
    /// 构建容器
    fn build(self) -> Result<Self::Container, ComponentError>;
    
    /// 添加组件注册
    fn register_component<T>(self, metadata: ComponentMetadata) -> Self
    where
        T: Component + 'static,
        Self: Sized;
    
    /// 添加单例注册
    fn register_singleton<T>(self, instance: Arc<T>) -> Self
    where
        T: Component + 'static,
        Self: Sized;
    
    /// 添加工厂注册
    fn register_factory<T>(self, factory: Box<dyn ComponentFactory>) -> Self
    where
        T: Component + 'static,
        Self: Sized;
    
    /// 添加扫描目标
    fn add_scan_target(self, target: String) -> Self
    where
        Self: Sized;
    
    /// 配置扫描器
    fn with_scanner(self, scanner: Box<dyn ComponentScanner>) -> Self
    where
        Self: Sized;
    
    /// 配置发现器
    fn with_discovery(self, discovery: Box<dyn ComponentDiscovery>) -> Self
    where
        Self: Sized;
}

/// 容器配置
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// 是否启用循环依赖检测
    pub enable_circular_dependency_detection: bool,
    /// 最大解析深度
    pub max_resolution_depth: usize,
    /// 解析超时时间（毫秒）
    pub resolution_timeout_ms: u64,
    /// 是否启用组件验证
    pub enable_component_validation: bool,
    /// 是否启用性能监控
    pub enable_performance_monitoring: bool,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            enable_circular_dependency_detection: true,
            max_resolution_depth: 100,
            resolution_timeout_ms: 5000,
            enable_component_validation: true,
            enable_performance_monitoring: false,
        }
    }
}

/// 容器统计信息
#[derive(Debug, Clone)]
pub struct ContainerStats {
    /// 已注册组件数量
    pub registered_components: usize,
    /// 已解析组件数量
    pub resolved_components: usize,
    /// 活跃单例数量
    pub active_singletons: usize,
    /// 解析总时间（毫秒）
    pub total_resolution_time_ms: u64,
    /// 平均解析时间（毫秒）
    pub average_resolution_time_ms: f64,
    /// 解析错误数量
    pub resolution_errors: usize,
}

impl Default for ContainerStats {
    fn default() -> Self {
        Self {
            registered_components: 0,
            resolved_components: 0,
            active_singletons: 0,
            total_resolution_time_ms: 0,
            average_resolution_time_ms: 0.0,
            resolution_errors: 0,
        }
    }
}
