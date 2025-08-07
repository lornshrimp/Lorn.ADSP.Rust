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
