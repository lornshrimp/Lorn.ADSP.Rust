//! # Component Macros
//!
//! 这个 crate 提供了用于自动组件注册和配置绑定的过程宏。
//!
//! ## 核心宏
//!
//! - [`component`] - 自动组件注册宏
//! - [`configurable`] - 自动配置绑定宏
//!
//! ## 使用示例
//!
//! ```rust
//! use component_macros::{component, configurable};
//! use infrastructure_common::{Component, Configurable};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Deserialize, Serialize)]
//! pub struct MyServiceConfig {
//!     pub enabled: bool,
//!     pub timeout: u64,
//! }
//!
//! #[component(singleton)]
//! #[configurable(path = "services.my_service")]
//! pub struct MyService {
//!     config: Option<MyServiceConfig>,
//! }
//! ```

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod component;
mod configurable;
mod lifecycle;
mod utils;

// Re-exports are not allowed in proc-macro crates

/// 自动组件注册宏
///
/// 这个宏会自动为结构体实现 `Component` trait，并注册到全局组件注册表中。
///
/// # 参数
///
/// - `singleton` - 单例生命周期（默认）
/// - `scoped` - 作用域生命周期
/// - `transient` - 瞬态生命周期
/// - `priority = N` - 组件优先级（默认为 0）
/// - `name = "custom_name"` - 自定义组件名称
///
/// # 示例
///
/// ```rust
/// #[component(singleton, priority = 100)]
/// pub struct MyService {
///     // 字段
/// }
/// ```
#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    component::component_impl(args, input)
}

/// 自动配置绑定宏
///
/// 这个宏会自动为结构体实现 `Configurable` trait，并提供配置绑定功能。
///
/// # 参数
///
/// - `path = "config.path"` - 配置路径
/// - `optional` - 配置是否可选（默认为必需）
/// - `default` - 使用默认配置
///
/// # 示例
///
/// ```rust
/// #[configurable(path = "services.my_service", optional)]
/// pub struct MyService {
///     // 字段
/// }
/// ```
#[proc_macro_attribute]
pub fn configurable(args: TokenStream, input: TokenStream) -> TokenStream {
    configurable::configurable_impl(args, input)
}

/// 生命周期管理宏
///
/// 这个宏用于定义组件的生命周期行为。
///
/// # 示例
///
/// ```rust
/// #[lifecycle(
///     on_start = "initialize",
///     on_stop = "cleanup",
///     depends_on = ["DatabaseService", "CacheService"]
/// )]
/// pub struct MyService {
///     // 字段
/// }
/// ```
#[proc_macro_attribute]
pub fn lifecycle(args: TokenStream, input: TokenStream) -> TokenStream {
    lifecycle::lifecycle_impl(args, input)
}

/// 组件派生宏
///
/// 自动为结构体实现基础的 `Component` trait。
///
/// # 示例
///
/// ```rust
/// #[derive(Component)]
/// pub struct MyService {
///     // 字段
/// }
/// ```
#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    component::derive_component_impl(input)
}

/// 可配置组件派生宏
///
/// 自动为结构体实现 `Configurable` trait。
///
/// # 示例
///
/// ```rust
/// #[derive(Configurable)]
/// #[configurable(path = "services.my_service")]
/// pub struct MyService {
///     config: MyServiceConfig,
/// }
/// ```
#[proc_macro_derive(Configurable, attributes(configurable))]
pub fn derive_configurable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    configurable::derive_configurable_impl(input)
}
