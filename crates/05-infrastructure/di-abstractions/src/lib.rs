//! # Dependency Injection Abstractions
//! 
//! 依赖注入抽象层，定义组件注册和依赖解析的核心接口。
//! 
//! ## 核心接口
//! 
//! - [`ComponentRegistry`] - 组件注册表接口
//! - [`ComponentScanner`] - 组件扫描器接口
//! - [`DependencyResolver`] - 依赖解析器接口
//! - [`ComponentFactory`] - 组件工厂接口

pub mod registry;
pub mod scanner;
pub mod resolver;
pub mod factory;
pub mod discovery;
pub mod container;

pub use registry::*;
pub use scanner::*;
pub use resolver::*;
pub use factory::*;
pub use discovery::*;
pub use container::*;
