//! 组件解析器抽象接口
//!
//! 提供依赖解析和组件实例化的能力

use infrastructure_common::{Component, DependencyError};
use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::sync::Arc;

/// 组件解析器 trait
///
/// 负责解析组件依赖并创建组件实例
#[async_trait]
pub trait ComponentResolver: Send + Sync {
    /// 解析指定类型的组件
    async fn resolve<T>(&self) -> Result<Arc<T>, DependencyError>
    where
        T: Component + 'static;
    
    /// 解析指定类型的组件（使用 TypeId）
    async fn resolve_by_type_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>, DependencyError>;
    
    /// 解析指定名称的组件
    async fn resolve_by_name(&self, name: &str) -> Result<Arc<dyn Any + Send + Sync>, DependencyError>;
    
    /// 检查是否可以解析指定类型
    fn can_resolve<T>(&self) -> bool
    where
        T: Component + 'static;
    
    /// 检查是否可以解析指定类型（使用 TypeId）
    fn can_resolve_by_type_id(&self, type_id: TypeId) -> bool;
    
    /// 检查是否可以解析指定名称的组件
    fn can_resolve_by_name(&self, name: &str) -> bool;
}

/// 解析上下文
#[derive(Debug, Clone)]
pub struct ResolveContext {
    /// 当前解析链，用于检测循环依赖
    pub resolution_chain: Vec<TypeId>,
    /// 解析选项
    pub options: ResolveOptions,
}

impl ResolveContext {
    /// 创建新的解析上下文
    pub fn new() -> Self {
        Self {
            resolution_chain: Vec::new(),
            options: ResolveOptions::default(),
        }
    }
    
    /// 添加类型到解析链
    pub fn push_type(&mut self, type_id: TypeId) -> Result<(), DependencyError> {
        if self.resolution_chain.contains(&type_id) {
            return Err(DependencyError::CircularDependency {
                dependency_chain: format!("检测到循环依赖: {:?}", self.resolution_chain),
            });
        }
        self.resolution_chain.push(type_id);
        Ok(())
    }
    
    /// 从解析链中移除类型
    pub fn pop_type(&mut self) {
        self.resolution_chain.pop();
    }
}

/// 解析选项
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// 是否允许创建临时实例
    pub allow_transient: bool,
    /// 最大递归深度
    pub max_depth: usize,
    /// 解析超时时间（毫秒）
    pub timeout_ms: u64,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            allow_transient: true,
            max_depth: 100,
            timeout_ms: 5000,
        }
    }
}
