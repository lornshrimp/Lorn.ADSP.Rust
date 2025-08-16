//! 组件反射和发现机制
//!
//! 提供编译时和运行时的组件发现能力

use crate::{Component, ComponentError, TypeInfo};
use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

/// 类型反射信息
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReflectionInfo {
    /// 类型ID
    pub type_id: TypeId,
    /// 类型名称
    pub type_name: &'static str,
    /// 模块路径
    pub module_path: &'static str,
    /// 是否为 trait object
    pub is_trait_object: bool,
    /// 泛型参数信息
    pub generic_params: Vec<TypeInfo>,
}

impl ReflectionInfo {
    /// 创建类型反射信息
    pub fn of<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            module_path: std::module_path!(),
            is_trait_object: false,
            generic_params: Vec::new(),
        }
    }

    /// 创建 trait object 反射信息
    pub fn of_trait<T: ?Sized + 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            module_path: std::module_path!(),
            is_trait_object: true,
            generic_params: Vec::new(),
        }
    }

    /// 添加泛型参数
    pub fn with_generic_param(mut self, param: TypeInfo) -> Self {
        self.generic_params.push(param);
        self
    }
}

/// 组件发现元数据
#[derive(Debug, Clone)]
pub struct DiscoveryMetadata {
    /// 反射信息
    pub reflection: ReflectionInfo,
    /// 组件名称
    pub component_name: String,
    /// 依赖类型列表
    pub dependencies: Vec<TypeInfo>,
    /// 提供的服务类型列表
    pub provides: Vec<TypeInfo>,
    /// 组件作用域
    pub scope: ComponentScope,
    /// 是否为单例
    pub singleton: bool,
    /// 组件标签
    pub tags: HashSet<String>,
    /// 启动顺序
    pub startup_order: i32,
}

impl DiscoveryMetadata {
    /// 创建发现元数据
    pub fn new(type_info: TypeInfo, name: impl Into<String>, dependencies: Vec<TypeInfo>) -> Self {
        Self {
            reflection: ReflectionInfo::of::<()>(), // 需要泛型支持
            component_name: name.into(),
            dependencies,
            provides: Vec::new(),
            scope: ComponentScope::Prototype,
            singleton: false,
            tags: HashSet::new(),
            startup_order: 0,
        }
    }

    /// 添加提供的服务
    pub fn provides<T: 'static>(mut self) -> Self {
        self.provides.push(TypeInfo::of::<T>());
        self
    }

    /// 设置作用域
    pub fn with_scope(mut self, scope: ComponentScope) -> Self {
        self.scope = scope;
        self
    }

    /// 设置单例
    pub fn singleton(mut self) -> Self {
        self.singleton = true;
        self.scope = ComponentScope::Singleton;
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// 设置启动顺序
    pub fn with_startup_order(mut self, order: i32) -> Self {
        self.startup_order = order;
        self
    }
}

/// 组件作用域
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComponentScope {
    /// 原型模式，每次请求创建新实例
    Prototype,
    /// 单例模式，全局唯一实例
    Singleton,
    /// 请求作用域，在同一请求中为单例
    Request,
    /// 会话作用域，在同一会话中为单例
    Session,
    /// 应用作用域，在应用生命周期内为单例
    Application,
}

/// 可发现的组件 trait
pub trait Discoverable: Component {
    /// 获取发现元数据
    fn discovery_metadata() -> DiscoveryMetadata
    where
        Self: Sized;

    /// 获取依赖列表
    fn get_dependencies(&self) -> Vec<TypeInfo>;

    /// 获取提供的服务列表
    fn get_provides(&self) -> Vec<TypeInfo> {
        Vec::new()
    }

    /// 检查是否可以提供指定类型的服务
    fn can_provide(&self, type_info: &TypeInfo) -> bool {
        self.get_provides().contains(type_info)
    }
}

/// 依赖关系信息
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// 依赖者类型
    pub dependent: TypeInfo,
    /// 被依赖者类型
    pub dependency: TypeInfo,
    /// 依赖关系类型
    pub relationship: DependencyRelationship,
    /// 是否为可选依赖
    pub optional: bool,
}

/// 依赖关系类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyRelationship {
    /// 构造函数依赖
    Constructor,
    /// 属性依赖
    Property,
    /// 方法依赖
    Method,
    /// 服务定位器依赖
    ServiceLocator,
}

/// 依赖关系图
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// 节点（组件）映射
    nodes: HashMap<TypeInfo, DiscoveryMetadata>,
    /// 边（依赖关系）列表
    edges: Vec<DependencyInfo>,
    /// 邻接表（用于快速查找）
    adjacency_list: HashMap<TypeInfo, Vec<TypeInfo>>,
    /// 反向邻接表（用于查找依赖者）
    reverse_adjacency_list: HashMap<TypeInfo, Vec<TypeInfo>>,
}

impl DependencyGraph {
    /// 创建新的依赖关系图
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency_list: HashMap::new(),
            reverse_adjacency_list: HashMap::new(),
        }
    }

    /// 添加组件节点
    pub fn add_component(&mut self, metadata: DiscoveryMetadata) {
        let type_info = TypeInfo::new(metadata.reflection.type_id, metadata.reflection.type_name);

        // 添加节点
        self.nodes.insert(type_info.clone(), metadata.clone());

        // 建立依赖关系
        for dep in &metadata.dependencies {
            self.add_dependency(
                type_info.clone(),
                dep.clone(),
                DependencyRelationship::Constructor,
                false,
            );
        }
    }

    /// 添加依赖关系
    pub fn add_dependency(
        &mut self,
        dependent: TypeInfo,
        dependency: TypeInfo,
        relationship: DependencyRelationship,
        optional: bool,
    ) {
        let dependency_info = DependencyInfo {
            dependent: dependent.clone(),
            dependency: dependency.clone(),
            relationship,
            optional,
        };

        self.edges.push(dependency_info);

        // 更新邻接表
        self.adjacency_list
            .entry(dependent.clone())
            .or_insert_with(Vec::new)
            .push(dependency.clone());

        self.reverse_adjacency_list
            .entry(dependency)
            .or_insert_with(Vec::new)
            .push(dependent);
    }

    /// 检测循环依赖
    pub fn detect_circular_dependencies(&self) -> Result<(), Vec<Vec<TypeInfo>>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut cycles = Vec::new();

        for node in self.nodes.keys() {
            if !visited.contains(node) {
                let mut path = Vec::new();
                self.dfs_detect_cycle(node, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        if cycles.is_empty() {
            Ok(())
        } else {
            Err(cycles)
        }
    }

    /// 深度优先搜索检测循环
    fn dfs_detect_cycle(
        &self,
        node: &TypeInfo,
        visited: &mut HashSet<TypeInfo>,
        rec_stack: &mut HashSet<TypeInfo>,
        path: &mut Vec<TypeInfo>,
        cycles: &mut Vec<Vec<TypeInfo>>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(dependencies) = self.adjacency_list.get(node) {
            for dep in dependencies {
                if rec_stack.contains(dep) {
                    // 找到循环，构建循环路径
                    if let Some(cycle_start) = path.iter().position(|n| n == dep) {
                        let cycle = path[cycle_start..].to_vec();
                        cycles.push(cycle);
                    }
                } else if !visited.contains(dep) {
                    self.dfs_detect_cycle(dep, visited, rec_stack, path, cycles);
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// 获取拓扑排序（启动顺序）
    pub fn topological_sort(&self) -> Result<Vec<TypeInfo>, ComponentError> {
        let mut in_degree = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        let mut result = Vec::new();

        // 计算入度
        for node in self.nodes.keys() {
            in_degree.insert(node.clone(), 0);
        }

        for edge in &self.edges {
            if !edge.optional {
                *in_degree.entry(edge.dependent.clone()).or_insert(0) += 1;
            }
        }

        // 找到入度为0的节点
        for (node, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(node.clone());
            }
        }

        // Kahn算法
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            if let Some(dependencies) = self.adjacency_list.get(&node) {
                for dep in dependencies {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            Err(ComponentError::CircularDependency {
                cycle: "检测到循环依赖".to_string(),
            })
        } else {
            Ok(result)
        }
    }

    /// 获取组件的直接依赖
    pub fn get_dependencies(&self, component: &TypeInfo) -> Vec<&TypeInfo> {
        self.adjacency_list
            .get(component)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// 获取依赖于指定组件的组件列表
    pub fn get_dependents(&self, component: &TypeInfo) -> Vec<&TypeInfo> {
        self.reverse_adjacency_list
            .get(component)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// 获取所有组件元数据
    pub fn get_all_components(&self) -> Vec<&DiscoveryMetadata> {
        self.nodes.values().collect()
    }

    /// 根据标签查找组件
    pub fn find_by_tag(&self, tag: &str) -> Vec<&DiscoveryMetadata> {
        self.nodes
            .values()
            .filter(|metadata| metadata.tags.contains(tag))
            .collect()
    }

    /// 根据作用域查找组件
    pub fn find_by_scope(&self, scope: &ComponentScope) -> Vec<&DiscoveryMetadata> {
        self.nodes
            .values()
            .filter(|metadata| &metadata.scope == scope)
            .collect()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// 组件发现器 trait
#[async_trait]
pub trait ComponentDiscovery: Send + Sync {
    /// 发现组件
    async fn discover_components(&self) -> Result<Vec<DiscoveryMetadata>, ComponentError>;

    /// 按类型发现组件
    async fn discover_by_type(
        &self,
        type_info: &TypeInfo,
    ) -> Result<Option<DiscoveryMetadata>, ComponentError>;

    /// 按标签发现组件
    async fn discover_by_tag(&self, tag: &str) -> Result<Vec<DiscoveryMetadata>, ComponentError>;

    /// 构建依赖关系图
    async fn build_dependency_graph(&self) -> Result<DependencyGraph, ComponentError>;
}

/// 基于反射的组件发现器实现
pub struct ReflectionComponentDiscovery {
    /// 组件注册表
    registry: Arc<dyn ComponentRegistry>,
}

impl ReflectionComponentDiscovery {
    /// 创建新的反射发现器
    pub fn new(registry: Arc<dyn ComponentRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ComponentDiscovery for ReflectionComponentDiscovery {
    async fn discover_components(&self) -> Result<Vec<DiscoveryMetadata>, ComponentError> {
        self.registry.get_all_discovery_metadata().await
    }

    async fn discover_by_type(
        &self,
        type_info: &TypeInfo,
    ) -> Result<Option<DiscoveryMetadata>, ComponentError> {
        self.registry.get_discovery_metadata(type_info).await
    }

    async fn discover_by_tag(&self, tag: &str) -> Result<Vec<DiscoveryMetadata>, ComponentError> {
        let all_components = self.discover_components().await?;
        Ok(all_components
            .into_iter()
            .filter(|metadata| metadata.tags.contains(tag))
            .collect())
    }

    async fn build_dependency_graph(&self) -> Result<DependencyGraph, ComponentError> {
        let mut graph = DependencyGraph::new();
        let components = self.discover_components().await?;

        for component in components {
            graph.add_component(component);
        }

        Ok(graph)
    }
}

/// 组件注册表 trait
#[async_trait]
pub trait ComponentRegistry: Send + Sync {
    /// 注册组件发现元数据
    async fn register_discovery_metadata(
        &self,
        metadata: DiscoveryMetadata,
    ) -> Result<(), ComponentError>;

    /// 获取组件发现元数据
    async fn get_discovery_metadata(
        &self,
        type_info: &TypeInfo,
    ) -> Result<Option<DiscoveryMetadata>, ComponentError>;

    /// 获取所有组件发现元数据
    async fn get_all_discovery_metadata(&self) -> Result<Vec<DiscoveryMetadata>, ComponentError>;

    /// 根据标签查找组件
    async fn find_by_tag(&self, tag: &str) -> Result<Vec<DiscoveryMetadata>, ComponentError>;

    /// 移除组件
    async fn remove_component(&self, type_info: &TypeInfo) -> Result<bool, ComponentError>;
}

/// 全局组件注册表
pub static GLOBAL_COMPONENT_REGISTRY: once_cell::sync::Lazy<
    Arc<dyn ComponentRegistry + Send + Sync>,
> = once_cell::sync::Lazy::new(|| Arc::new(InMemoryComponentRegistry::new()));

/// 内存中的组件注册表实现
pub struct InMemoryComponentRegistry {
    components: tokio::sync::RwLock<HashMap<TypeInfo, DiscoveryMetadata>>,
}

impl InMemoryComponentRegistry {
    /// 创建新的内存注册表
    pub fn new() -> Self {
        Self {
            components: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ComponentRegistry for InMemoryComponentRegistry {
    async fn register_discovery_metadata(
        &self,
        metadata: DiscoveryMetadata,
    ) -> Result<(), ComponentError> {
        let type_info = TypeInfo::new(metadata.reflection.type_id, metadata.reflection.type_name);

        let mut components = self.components.write().await;
        components.insert(type_info, metadata);
        Ok(())
    }

    async fn get_discovery_metadata(
        &self,
        type_info: &TypeInfo,
    ) -> Result<Option<DiscoveryMetadata>, ComponentError> {
        let components = self.components.read().await;
        Ok(components.get(type_info).cloned())
    }

    async fn get_all_discovery_metadata(&self) -> Result<Vec<DiscoveryMetadata>, ComponentError> {
        let components = self.components.read().await;
        Ok(components.values().cloned().collect())
    }

    async fn find_by_tag(&self, tag: &str) -> Result<Vec<DiscoveryMetadata>, ComponentError> {
        let components = self.components.read().await;
        Ok(components
            .values()
            .filter(|metadata| metadata.tags.contains(tag))
            .cloned()
            .collect())
    }

    async fn remove_component(&self, type_info: &TypeInfo) -> Result<bool, ComponentError> {
        let mut components = self.components.write().await;
        Ok(components.remove(type_info).is_some())
    }
}
// 为 ComponentScope 实现 FromStr trait 以支持字符串解析
impl std::str::FromStr for ComponentScope {
    type Err = ComponentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "prototype" => Ok(ComponentScope::Prototype),
            "singleton" => Ok(ComponentScope::Singleton),
            "request" => Ok(ComponentScope::Request),
            "session" => Ok(ComponentScope::Session),
            "application" => Ok(ComponentScope::Application),
            _ => Err(ComponentError::ParseError {
                message: format!("Unknown component scope: {}", s),
            }),
        }
    }
}
