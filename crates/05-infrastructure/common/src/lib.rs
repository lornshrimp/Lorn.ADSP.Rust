//! # Infrastructure Common
//!
//! 这个 crate 提供了 Lorn ADSP 平台基础设施层的公共 traits 和工具。
//!
//! ## 核心组件
//!
//! - [`Component`] - 组件基础 trait
//! - [`Configurable`] - 可配置组件 trait  
//! - [`HealthCheckable`] - 健康检查 trait
//! - [`ComponentConventions`] - 组件约定规范
//! - [`Lifecycle`] - 组件生命周期管理
//!
//! ## 设计原则
//!
//! - 基于 Rust 类型系统的编译时安全
//! - 异步优先的设计理念
//! - 约定优于配置
//! - 可扩展的组件发现机制

pub mod component;
pub mod configuration;
pub mod conventions;
pub mod discovery;
pub mod errors;
pub mod health;
pub mod lifecycle;
pub mod metadata;

pub use component::*;
pub use configuration::*;
pub use conventions::*;
pub use discovery::*;
pub use errors::*;
pub use health::*;
pub use lifecycle::*;
pub use metadata::*;

/// 全局组件注册表
static GLOBAL_COMPONENT_REGISTRY: once_cell::sync::Lazy<
    parking_lot::RwLock<Option<std::sync::Arc<dyn GlobalComponentRegistry>>>
> = once_cell::sync::Lazy::new(|| parking_lot::RwLock::new(None));

/// 全局组件注册表 trait
pub trait GlobalComponentRegistry: Send + Sync {
    /// 注册组件描述符
    fn register_component_descriptor(&self, descriptor: ComponentDescriptor) -> Result<(), crate::errors::InfrastructureError>;
    
    /// 获取所有注册的组件描述符
    fn get_all_descriptors(&self) -> Vec<ComponentDescriptor>;
}

/// 获取全局组件注册表
pub fn get_global_component_registry() -> Option<std::sync::Arc<dyn GlobalComponentRegistry>> {
    GLOBAL_COMPONENT_REGISTRY.read().clone()
}

/// 设置全局组件注册表
pub fn set_global_component_registry(registry: std::sync::Arc<dyn GlobalComponentRegistry>) {
    *GLOBAL_COMPONENT_REGISTRY.write() = Some(registry);
}
