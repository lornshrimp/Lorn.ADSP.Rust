//! 组件生命周期管理

use crate::errors::LifecycleError;
use async_trait::async_trait;

/// 组件生命周期类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lifetime {
    /// 单例模式 - 整个应用生命周期内只创建一个实例
    Singleton,
    /// 作用域模式 - 在同一作用域内共享实例
    Scoped,
    /// 瞬时模式 - 每次请求都创建新实例
    Transient,
}

impl Default for Lifetime {
    fn default() -> Self {
        Self::Transient
    }
}

/// 生命周期标记 trait
pub trait LifecycleMarker: Send + Sync + 'static {}

/// 单例生命周期标记
pub trait Singleton: LifecycleMarker {}

/// 作用域生命周期标记  
pub trait Scoped: LifecycleMarker {}

/// 瞬时生命周期标记
pub trait Transient: LifecycleMarker {}

/// 组件作用域
#[derive(Debug, Clone)]
pub struct Scope {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Scope {
    /// 创建新作用域
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            created_at: chrono::Utc::now(),
        }
    }

    /// 创建根作用域
    pub fn root() -> Self {
        Self::new("root")
    }

    /// 创建子作用域
    pub fn child(&self, name: impl Into<String>) -> Self {
        Self::new(format!("{}.{}", self.name, name.into()))
    }
}

/// 作用域守卫
pub struct ScopeGuard {
    scope: Scope,
    _cleanup: Box<dyn FnOnce() + Send>,
}

impl ScopeGuard {
    /// 创建新的作用域守卫
    pub fn new(scope: Scope, cleanup: Box<dyn FnOnce() + Send>) -> Self {
        Self {
            scope,
            _cleanup: cleanup,
        }
    }

    /// 获取作用域
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        // cleanup 会在这里自动执行
    }
}

/// 生命周期管理器
#[async_trait]
pub trait LifecycleManager: Send + Sync {
    /// 确定组件的生命周期
    fn determine_lifetime(&self, type_info: &crate::metadata::TypeInfo) -> Lifetime;

    /// 创建新作用域
    async fn create_scope(&self, name: impl Into<String> + Send) -> Result<Scope, LifecycleError>;

    /// 管理作用域
    async fn manage_scope(&self, scope: Scope) -> Result<ScopeGuard, LifecycleError>;

    /// 销毁作用域
    async fn destroy_scope(&self, scope_id: uuid::Uuid) -> Result<(), LifecycleError>;
}

/// 默认生命周期管理器实现
#[derive(Debug)]
pub struct DefaultLifecycleManager {
    active_scopes: dashmap::DashMap<uuid::Uuid, Scope>,
}

impl DefaultLifecycleManager {
    /// 创建新的生命周期管理器
    pub fn new() -> Self {
        Self {
            active_scopes: dashmap::DashMap::new(),
        }
    }
}

impl Default for DefaultLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LifecycleManager for DefaultLifecycleManager {
    fn determine_lifetime(&self, type_info: &crate::metadata::TypeInfo) -> Lifetime {
        // 基于命名约定确定生命周期
        let type_name = &type_info.name;

        if type_name.ends_with("Service") || type_name.ends_with("Manager") {
            Lifetime::Singleton
        } else if type_name.ends_with("Provider") {
            Lifetime::Scoped
        } else {
            Lifetime::Transient
        }
    }

    async fn create_scope(&self, name: impl Into<String> + Send) -> Result<Scope, LifecycleError> {
        let scope = Scope::new(name);
        self.active_scopes.insert(scope.id, scope.clone());
        Ok(scope)
    }

    async fn manage_scope(&self, scope: Scope) -> Result<ScopeGuard, LifecycleError> {
        let scope_id = scope.id;
        let active_scopes = self.active_scopes.clone();

        let cleanup = Box::new(move || {
            active_scopes.remove(&scope_id);
        });

        Ok(ScopeGuard::new(scope, cleanup))
    }

    async fn destroy_scope(&self, scope_id: uuid::Uuid) -> Result<(), LifecycleError> {
        self.active_scopes.remove(&scope_id);
        Ok(())
    }
}

/// 组件生命周期状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 错误状态
    Error,
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self::Uninitialized
    }
}

/// 组件生命周期管理 trait
#[async_trait]
pub trait Lifecycle: Send + Sync {
    /// 组件启动
    async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 组件停止
    async fn on_stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 获取生命周期状态
    fn get_lifecycle_state(&self) -> LifecycleState;

    /// 是否可以启动
    fn can_start(&self) -> bool {
        matches!(
            self.get_lifecycle_state(),
            LifecycleState::Uninitialized | LifecycleState::Stopped
        )
    }

    /// 是否可以停止
    fn can_stop(&self) -> bool {
        matches!(self.get_lifecycle_state(), LifecycleState::Running)
    }
}

/// 依赖感知 trait
pub trait DependencyAware {
    /// 获取依赖列表
    fn get_dependencies(&self) -> Vec<String>;

    /// 是否可以在没有依赖的情况下启动
    fn can_start_without_dependencies(&self) -> bool {
        true
    }
}
