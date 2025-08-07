//! 组件工厂抽象接口
//!
//! 提供组件实例创建的工厂模式支持

use infrastructure_common::{Component, DependencyError};
use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::sync::Arc;

/// 组件工厂 trait
///
/// 用于创建组件实例
#[async_trait]
pub trait ComponentFactory: Send + Sync {
    /// 创建组件实例
    async fn create(&self, dependencies: Vec<Arc<dyn Any + Send + Sync>>) -> Result<Arc<dyn Any + Send + Sync>, DependencyError>;
    
    /// 获取工厂支持的组件类型
    fn component_type(&self) -> TypeId;
    
    /// 获取工厂名称
    fn name(&self) -> &str;
    
    /// 获取所需的依赖类型
    fn dependencies(&self) -> Vec<TypeId>;
}

/// 简单工厂 trait
///
/// 用于创建无依赖的组件实例
#[async_trait]
pub trait SimpleFactory<T>: Send + Sync
where
    T: Component + 'static,
{
    /// 创建组件实例
    async fn create(&self) -> Result<Arc<T>, DependencyError>;
}

/// 带依赖的工厂 trait
///
/// 用于创建有依赖的组件实例
#[async_trait]
pub trait DependentFactory<T>: Send + Sync
where
    T: Component + 'static,
{
    /// 依赖类型
    type Dependencies;
    
    /// 创建组件实例
    async fn create(&self, dependencies: Self::Dependencies) -> Result<Arc<T>, DependencyError>;
}

/// 工厂注册器
pub trait FactoryRegistry: Send + Sync {
    /// 注册简单工厂
    fn register_simple<T, F>(&mut self, factory: F) -> Result<(), DependencyError>
    where
        T: Component + 'static,
        F: SimpleFactory<T> + 'static;
    
    /// 注册依赖工厂
    fn register_dependent<T, F, D>(&mut self, factory: F) -> Result<(), DependencyError>
    where
        T: Component + 'static,
        F: DependentFactory<T, Dependencies = D> + 'static,
        D: Send + Sync + 'static;
    
    /// 获取工厂
    fn get_factory(&self, type_id: TypeId) -> Option<&dyn ComponentFactory>;
}

/// Lambda 工厂包装器
pub struct LambdaFactory<T, F>
where
    T: Component + 'static,
    F: Fn() -> Result<Arc<T>, DependencyError> + Send + Sync + 'static,
{
    pub factory_fn: F,
    pub component_type: std::marker::PhantomData<T>,
}

impl<T, F> LambdaFactory<T, F>
where
    T: Component + 'static,
    F: Fn() -> Result<Arc<T>, DependencyError> + Send + Sync + 'static,
{
    pub fn new(factory_fn: F) -> Self {
        Self {
            factory_fn,
            component_type: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, F> SimpleFactory<T> for LambdaFactory<T, F>
where
    T: Component + 'static,
    F: Fn() -> Result<Arc<T>, DependencyError> + Send + Sync + 'static,
{
    async fn create(&self) -> Result<Arc<T>, DependencyError> {
        (self.factory_fn)()
    }
}
