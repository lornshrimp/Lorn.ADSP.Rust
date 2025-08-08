//! # Configuration Implementation
//! 
//! 配置管理的具体实现，提供各种配置源和管理功能。
//! 
//! ## 主要组件
//! 
//! - [`AdSystemConfigManager`] - 主配置管理器
//! - [`TomlConfigProvider`] - TOML 配置提供者
//! - [`JsonConfigProvider`] - JSON 配置提供者
//! - [`EnvironmentConfigProvider`] - 环境变量配置提供者
//! - [`ConfigValidationManager`] - 配置验证管理器
//! - [`TypedConfigBinder`] - 类型化配置绑定器

pub mod manager;
pub mod providers;
pub mod validation;
pub mod binder;
pub mod watcher;

pub use manager::*;
pub use providers::*;
pub use validation::*;
pub use watcher::*;
