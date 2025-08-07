//! # Configuration Abstractions
//! 
//! 配置管理抽象层，定义配置管理的核心接口和约定。
//! 
//! ## 核心接口
//! 
//! - [`ConfigProvider`] - 配置提供者接口
//! - [`ConfigManager`] - 配置管理器接口
//! - [`ConfigWatcher`] - 配置监控接口
//! - [`ConfigValidator`] - 配置验证接口

pub mod provider;
pub mod manager;
pub mod watcher;
pub mod validator;
pub mod events;

pub use provider::*;
pub use manager::*; 
pub use watcher::*;
pub use validator::*;
pub use events::*;
