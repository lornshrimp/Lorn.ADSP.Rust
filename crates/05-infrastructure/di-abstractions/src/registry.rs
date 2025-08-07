//! 组件注册表抽象接口

use async_trait::async_trait;
use infrastructure_common::{
    Component, ComponentDescriptor, DependencyError, Lifetime, Scope,
};
use std::any::{Any, TypeId};
use std::sync::Arc;

/// 组件注册表 trait
/// 
/// 提供组件注册和解析的核心接口
#[async_trait]
pub trait ComponentRegistry: Send + Sync {
    /// 注册组件
    async fn register_component<T>(&mut self, lifetime: Lifetime) -> Result<(), DependencyError>
    where
        T: Component + 'static;
    
    /// 注册组件实例
    async fn register_instance<T>(&mut self, instance: T) -> Result<(), DependencyError>
    where
        T: Component + 'static;
    
    /// 注册组件工厂
    async fn register_factory<T, F>(&mut self, factory: F, lifetime: Lifetime) -> Result<(), DependencyError>
    where
        T: Component + 'static,
        F: Fn() -> Result<T, DependencyError> + Send + Sync + 'static;
    
    /// 解析组件
    async fn resolve<T>(&self) -> Result<Arc<T>, DependencyError>
    where
        T: Component + 'static;
    
    /// 解析作用域组件
    async fn resolve_scoped<T>(&self, scope: &Scope) -> Result<Arc<T>, DependencyError>
    where
        T: Component + 'static;
    
    /// 解析所有实现指定 trait 的组件
    async fn resolve_all<T>(&self) -> Result<Vec<Arc<T>>, DependencyError>
    where
        T: Component + 'static;
    
    /// 检查组件是否已注册
    fn is_registered<T>(&self) -> bool
    where
        T: 'static;
    
    /// 检查组件是否已注册（通过 TypeId）
    fn is_registered_by_type_id(&self, type_id: TypeId) -> bool;
    
    /// 获取所有已注册的组件描述符
    fn get_registered_components(&self) -> Vec<ComponentDescriptor>;
    
    /// 注册所有组件
    async fn register_all_components(&mut self) -> Result<(), DependencyError>;
    
    /// 验证依赖关系
    async fn validate_dependencies(&self) -> Result<(), DependencyError>;
    
    /// 清理已注册的组件
    async fn clear(&mut self) -> Result<(), DependencyError>;
}

/// 可注册组件 trait
pub trait RegisterableComponent: Component {
    /// 获取依赖类型列表
    fn dependencies() -> Vec<TypeId>;
    
    /// 创建组件实例
    fn create(dependencies: Vec<Arc<dyn Any + Send + Sync>>) -> Result<Self, DependencyError>
    where
        Self: Sized;
    
    /// 获取默认生命周期
    fn default_lifetime() -> Lifetime {
        Lifetime::Transient
    }
}

/// 组件注册信息
#[derive(Clone)]
pub struct ComponentRegistration {
    /// 组件描述符
    pub descriptor: ComponentDescriptor,
    /// 组件工厂
    pub factory: ComponentFactoryFn,
    /// 依赖列表
    pub dependencies: Vec<TypeId>,
    /// 是否已创建实例（仅用于单例模式）
    pub instance_created: bool,
}

impl std::fmt::Debug for ComponentRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentRegistration")
            .field("descriptor", &self.descriptor)
            .field("dependencies", &self.dependencies)
            .field("instance_created", &self.instance_created)
            .field("factory", &"<function>")
            .finish()
    }
}

/// 组件工厂函数类型
pub type ComponentFactoryFn = Arc<
    dyn Fn(Vec<Arc<dyn Any + Send + Sync>>) -> Result<Arc<dyn Any + Send + Sync>, DependencyError>
        + Send
        + Sync,
>;

/// 作用域组件实例
#[derive(Debug)]
pub struct ScopedInstance {
    /// 实例
    pub instance: Arc<dyn Any + Send + Sync>,
    /// 作用域
    pub scope: Scope,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 依赖图节点
#[derive(Debug, Clone)]
pub struct DependencyGraphNode {
    /// 组件类型ID
    pub type_id: TypeId,
    /// 组件名称
    pub name: String,
    /// 依赖的类型ID列表
    pub dependencies: Vec<TypeId>,
    /// 依赖深度
    pub depth: usize,
}

/// 循环依赖检测器
pub trait CircularDependencyDetector: Send + Sync {
    /// 检测循环依赖
    fn detect_circular_dependencies(&self, graph: &[DependencyGraphNode]) -> Result<(), DependencyError>;
    
    /// 构建依赖图
    fn build_dependency_graph(&self, registrations: &[ComponentRegistration]) -> Vec<DependencyGraphNode>;
}

/// 默认循环依赖检测器
#[derive(Debug, Default)]
pub struct DefaultCircularDependencyDetector;

impl CircularDependencyDetector for DefaultCircularDependencyDetector {
    fn detect_circular_dependencies(&self, graph: &[DependencyGraphNode]) -> Result<(), DependencyError> {
        // 使用深度优先搜索检测循环依赖
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();
        
        for node in graph {
            if !visited.contains(&node.type_id) {
                self.dfs_check(node.type_id, graph, &mut visited, &mut visiting)?;
            }
        }
        
        Ok(())
    }
    
    fn build_dependency_graph(&self, registrations: &[ComponentRegistration]) -> Vec<DependencyGraphNode> {
        registrations
            .iter()
            .map(|reg| DependencyGraphNode {
                type_id: reg.descriptor.type_id,
                name: reg.descriptor.name.clone(),
                dependencies: reg.dependencies.clone(),
                depth: 0, // 会在后续计算中更新
            })
            .collect()
    }
}

impl DefaultCircularDependencyDetector {
    fn dfs_check(
        &self,
        current: TypeId,
        graph: &[DependencyGraphNode],
        visited: &mut std::collections::HashSet<TypeId>,
        visiting: &mut std::collections::HashSet<TypeId>,
    ) -> Result<(), DependencyError> {
        if visiting.contains(&current) {
            // 检测到循环依赖
            let chain = visiting
                .iter()
                .map(|id| format!("{:?}", id))
                .collect::<Vec<_>>()
                .join(" -> ");
            
            return Err(DependencyError::CircularDependency {
                dependency_chain: format!("{} -> {:?}", chain, current),
            });
        }
        
        if visited.contains(&current) {
            return Ok(());
        }
        
        visiting.insert(current);
        
        // 查找当前节点的依赖
        if let Some(node) = graph.iter().find(|n| n.type_id == current) {
            for dep in &node.dependencies {
                self.dfs_check(*dep, graph, visited, visiting)?;
            }
        }
        
        visiting.remove(&current);
        visited.insert(current);
        
        Ok(())
    }
}
