//! # 基础设施组合层
//!
//! 这个 crate 是广告系统基础设施的组合层，负责将各个基础设施组件组合成一个
//! 完整的、可运行的系统。
//!
//! ## 主要功能
//!
//! - **基础设施构建器**: 使用构建者模式组装基础设施组件
//! - **配置源管理**: 统一管理多种类型的配置源
//! - **组件扫描发现**: 自动化组件发现和注册
//! - **生命周期管理**: 管理整个基础设施的启动和关闭
//!
//! ## 基本使用
//!
//! ```rust,no_run
//! use infrastructure_composition::InfrastructureBuilder;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 创建并配置基础设施
//!     let infrastructure = InfrastructureBuilder::new()
//!         .build()
//!         .await?;
//!
//!     // 启动基础设施
//!     infrastructure.start().await?;
//!
//!     // 使用基础设施
//!     let config_value: String = infrastructure.get_config("app.name").await?;
//!     println!("应用名称: {}", config_value);
//!
//!     // 停止基础设施
//!     infrastructure.stop().await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod builder;
pub mod component_scanner;
pub mod config_sources;
pub mod enhanced_component_scanner;
pub mod infrastructure;

// 重新导出主要类型
pub use builder::InfrastructureBuilder;
pub use component_scanner::{
    AdvancedComponentManager, ComponentDiscoveryStrategy, ComponentLifecycle,
    ComponentRegistration, ComponentScannerBuilder, ComponentScannerImpl,
};
// pub use enhanced_component_scanner::{
//     ComponentFilter, ComponentInterceptor, LoggingInterceptor, 
//     ScopeFilter, NameFilter, ComponentScannerImpl as EnhancedComponentScannerImpl,
//     AdvancedComponentManager as EnhancedAdvancedComponentManager, 
//     ComponentDiscoveryStrategy as EnhancedComponentDiscoveryStrategy, 
//     AdvancedComponentMetadata,
// };
pub use config_sources::{
    ConfigSourceDescriptor, ConfigSourceManagerBuilder, ConfigSourceOptions, ConfigSourceType,
    ExtendedConfigSourceManager,
};
pub use infrastructure::{AdSystemInfrastructure, InfrastructureMetrics, InfrastructureStatus};

// 重新导出错误类型
pub use infrastructure_common::InfrastructureError;
