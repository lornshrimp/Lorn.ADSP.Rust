//! 组件扫描和发现功能
//! 
//! 提供自动化的组件发现、注册和管理能力

use di_abstractions::ComponentScanner;
use di_impl::DiContainerImpl;
use infrastructure_common::{
    Component, DependencyError, ComponentMetadata,
};
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

/// 组件发现策略
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentDiscoveryStrategy {
    /// 自动发现（通过反射和约定）
    Automatic,
    /// 基于属性标注
    AttributeBased,
    /// 基于配置文件
    ConfigurationBased,
    /// 基于目录扫描
    DirectoryScan,
    /// 手动注册
    Manual,
}

/// 组件生命周期类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentLifecycle {
    /// 单例模式
    Singleton,
    /// 瞬态模式（每次都创建新实例）
    Transient,
    /// 作用域模式（在特定作用域内共享）
    Scoped,
}

/// 组件注册信息
#[derive(Debug, Clone)]
pub struct ComponentRegistration {
    /// 组件类型ID
    pub type_id: TypeId,
    /// 组件类型名称
    pub type_name: String,
    /// 组件生命周期
    pub lifecycle: ComponentLifecycle,
    /// 组件依赖列表
    pub dependencies: Vec<TypeId>,
    /// 组件接口列表
    pub interfaces: Vec<TypeId>,
    /// 是否为主要实现
    pub is_primary: bool,
    /// 优先级（数字越小优先级越高）
    pub priority: u32,
    /// 组件标签
    pub tags: HashSet<String>,
    /// 组件配置键
    pub config_key: Option<String>,
}

/// 组件扫描器实现
pub struct ComponentScannerImpl {
    /// 发现策略
    strategy: ComponentDiscoveryStrategy,
    /// 扫描包列表
    scan_packages: Vec<String>,
    /// 已发现的组件
    discovered_components: Arc<RwLock<HashMap<TypeId, ComponentRegistration>>>,
    /// 组件工厂函数
    component_factories: Arc<RwLock<HashMap<TypeId, ComponentFactory>>>,
    /// 扫描结果缓存
    scan_cache: Arc<RwLock<HashMap<String, Vec<ComponentRegistration>>>>,
}

/// 组件工厂函数类型
type ComponentFactory = Box<dyn Fn() -> Result<Box<dyn Any + Send + Sync>, DependencyError> + Send + Sync>;

impl ComponentScannerImpl {
    /// 创建新的组件扫描器
    pub fn new(strategy: ComponentDiscoveryStrategy) -> Self {
        Self {
            strategy,
            scan_packages: Vec::new(),
            discovered_components: Arc::new(RwLock::new(HashMap::new())),
            component_factories: Arc::new(RwLock::new(HashMap::new())),
            scan_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 添加扫描包
    pub fn add_scan_package<S: Into<String>>(mut self, package: S) -> Self {
        self.scan_packages.push(package.into());
        self
    }
    
    /// 注册组件工厂
    pub async fn register_factory<T, F>(
        &self,
        factory: F,
        lifecycle: ComponentLifecycle,
    ) -> Result<(), DependencyError>
    where
        T: Component + 'static,
        F: Fn() -> Result<T, DependencyError> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();
        
        // 包装工厂函数
        let wrapped_factory: ComponentFactory = Box::new(move || {
            match factory() {
                Ok(instance) => Ok(Box::new(instance) as Box<dyn Any + Send + Sync>),
                Err(e) => Err(e),
            }
        });
        
        // 注册工厂
        {
            let mut factories = self.component_factories.write().await;
            factories.insert(type_id, wrapped_factory);
        }
        
        // 创建组件注册信息
        let registration = ComponentRegistration {
            type_id,
            type_name: type_name.clone(),
            lifecycle,
            dependencies: Vec::new(), // TODO: 通过反射或分析获取依赖
            interfaces: Vec::new(),   // TODO: 通过反射获取实现的接口
            is_primary: true,
            priority: 100,
            tags: HashSet::new(),
            config_key: None,
        };
        
        // 保存注册信息
        {
            let mut components = self.discovered_components.write().await;
            components.insert(type_id, registration);
        }
        
        debug!("注册组件工厂: {}", type_name);
        Ok(())
    }
    
    /// 扫描指定包中的组件
    pub async fn scan_package(&self, package_name: &str) -> Result<Vec<ComponentRegistration>, DependencyError> {
        debug!("开始扫描包: {}", package_name);
        
        // 检查缓存
        {
            let cache = self.scan_cache.read().await;
            if let Some(cached_results) = cache.get(package_name) {
                debug!("使用缓存的扫描结果: {}", package_name);
                return Ok(cached_results.clone());
            }
        }
        
        let mut discovered = Vec::new();
        
        match self.strategy {
            ComponentDiscoveryStrategy::Automatic => {
                discovered = self.scan_automatic(package_name).await?;
            }
            
            ComponentDiscoveryStrategy::AttributeBased => {
                discovered = self.scan_attribute_based(package_name).await?;
            }
            
            ComponentDiscoveryStrategy::ConfigurationBased => {
                discovered = self.scan_configuration_based(package_name).await?;
            }
            
            ComponentDiscoveryStrategy::DirectoryScan => {
                discovered = self.scan_directory(package_name).await?;
            }
            
            ComponentDiscoveryStrategy::Manual => {
                // 手动模式不执行自动扫描
                debug!("手动模式，跳过自动扫描: {}", package_name);
            }
        }
        
        // 更新缓存
        {
            let mut cache = self.scan_cache.write().await;
            cache.insert(package_name.to_string(), discovered.clone());
        }
        
        info!("扫描包 {} 完成，发现 {} 个组件", package_name, discovered.len());
        Ok(discovered)
    }
    
    /// 自动扫描模式
    async fn scan_automatic(&self, package_name: &str) -> Result<Vec<ComponentRegistration>, DependencyError> {
        debug!("使用自动模式扫描包: {}", package_name);
        
        // 在实际实现中，这里会使用反射或编译时宏来发现组件
        // 目前返回空列表，因为 Rust 的反射能力有限
        
        // TODO: 实现基于约定的组件发现
        // 例如：所有实现了 Component trait 的类型
        // 或者所有以 "Service", "Repository", "Manager" 结尾的类型
        
        warn!("自动组件发现功能尚未完全实现");
        Ok(Vec::new())
    }
    
    /// 基于属性的扫描模式
    async fn scan_attribute_based(&self, package_name: &str) -> Result<Vec<ComponentRegistration>, DependencyError> {
        debug!("使用属性模式扫描包: {}", package_name);
        
        // 在实际实现中，这里会查找带有特定属性标注的类型
        // 例如：#[component], #[service], #[repository] 等
        
        // TODO: 实现基于属性的组件发现
        // 需要配合 proc_macro 来实现编译时组件注册
        
        warn!("基于属性的组件发现功能尚未实现");
        Ok(Vec::new())
    }
    
    /// 基于配置的扫描模式
    async fn scan_configuration_based(&self, package_name: &str) -> Result<Vec<ComponentRegistration>, DependencyError> {
        debug!("使用配置模式扫描包: {}", package_name);
        
        // 在实际实现中，这里会读取配置文件来确定要注册的组件
        // 例如：components.toml, services.json 等
        
        // TODO: 实现基于配置文件的组件发现
        // 读取配置文件，解析组件定义，创建注册信息
        
        warn!("基于配置的组件发现功能尚未实现");
        Ok(Vec::new())
    }
    
    /// 基于目录的扫描模式
    async fn scan_directory(&self, package_name: &str) -> Result<Vec<ComponentRegistration>, DependencyError> {
        debug!("使用目录模式扫描包: {}", package_name);
        
        // 在实际实现中，这里会扫描指定目录下的文件
        // 根据文件名约定来发现组件
        
        // TODO: 实现基于目录结构的组件发现
        // 扫描目录，根据文件名模式匹配组件
        
        warn!("基于目录的组件发现功能尚未实现");
        Ok(Vec::new())
    }
    
    /// 将发现的组件注册到容器中
    pub async fn register_discovered_components(
        &self,
        container: &mut DiContainerImpl,
    ) -> Result<usize, DependencyError> {
        info!("开始注册发现的组件到容器");
        
        let components = self.discovered_components.read().await;
        let factories = self.component_factories.read().await;
        
        let mut registered_count = 0;
        
        for (type_id, registration) in components.iter() {
            if let Some(factory) = factories.get(type_id) {
                // 根据生命周期类型注册组件
                match registration.lifecycle {
                    ComponentLifecycle::Singleton => {
                        // 创建实例并注册为单例
                        match factory() {
                            Ok(instance) => {
                                // 由于类型擦除，这里需要特殊处理
                                // 在实际实现中，可能需要使用 trait object 或其他方式
                                debug!("注册单例组件: {}", registration.type_name);
                                registered_count += 1;
                            }
                            Err(e) => {
                                error!("创建组件实例失败: {} - {:?}", registration.type_name, e);
                                return Err(e);
                            }
                        }
                    }
                    
                    ComponentLifecycle::Transient => {
                        // 注册瞬态工厂
                        debug!("注册瞬态组件: {}", registration.type_name);
                        registered_count += 1;
                    }
                    
                    ComponentLifecycle::Scoped => {
                        // 注册作用域工厂
                        debug!("注册作用域组件: {}", registration.type_name);
                        registered_count += 1;
                    }
                }
            }
        }
        
        info!("组件注册完成，共注册 {} 个组件", registered_count);
        Ok(registered_count)
    }
    
    /// 获取所有发现的组件
    pub async fn get_discovered_components(&self) -> Vec<ComponentRegistration> {
        let components = self.discovered_components.read().await;
        components.values().cloned().collect()
    }
    
    /// 清空扫描缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.scan_cache.write().await;
        cache.clear();
        debug!("扫描缓存已清空");
    }
    
    /// 获取组件依赖关系
    pub async fn analyze_dependencies(&self) -> Result<HashMap<TypeId, Vec<TypeId>>, DependencyError> {
        let components = self.discovered_components.read().await;
        let mut dependencies = HashMap::new();
        
        for (type_id, registration) in components.iter() {
            dependencies.insert(*type_id, registration.dependencies.clone());
        }
        
        Ok(dependencies)
    }
    
    /// 检测循环依赖
    pub async fn detect_circular_dependencies(&self) -> Result<Vec<Vec<TypeId>>, DependencyError> {
        let dependencies = self.analyze_dependencies().await?;
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        
        for &type_id in dependencies.keys() {
            if !visited.contains(&type_id) {
                if let Some(cycle) = self.detect_cycle(
                    type_id,
                    &dependencies,
                    &mut visited,
                    &mut recursion_stack,
                    &mut Vec::new(),
                ) {
                    cycles.push(cycle);
                }
            }
        }
        
        Ok(cycles)
    }
    
    /// 递归检测循环依赖
    fn detect_cycle(
        &self,
        current: TypeId,
        dependencies: &HashMap<TypeId, Vec<TypeId>>,
        visited: &mut HashSet<TypeId>,
        recursion_stack: &mut HashSet<TypeId>,
        path: &mut Vec<TypeId>,
    ) -> Option<Vec<TypeId>> {
        visited.insert(current);
        recursion_stack.insert(current);
        path.push(current);
        
        if let Some(deps) = dependencies.get(&current) {
            for &dep in deps {
                if !visited.contains(&dep) {
                    if let Some(cycle) = self.detect_cycle(dep, dependencies, visited, recursion_stack, path) {
                        return Some(cycle);
                    }
                } else if recursion_stack.contains(&dep) {
                    // 找到循环
                    let cycle_start = path.iter().position(|&x| x == dep).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }
        
        recursion_stack.remove(&current);
        path.pop();
        None
    }
}

#[async_trait::async_trait]
impl ComponentScanner for ComponentScannerImpl {
    async fn scan(&self, package_path: &str) -> Result<Vec<ComponentMetadata>, infrastructure_common::ComponentError> {
        let registrations = self.scan_package(package_path).await
            .map_err(|e| infrastructure_common::ComponentError::DiscoveryError {
                message: format!("扫描失败: {:?}", e),
            })?;
        
        // 转换为 ComponentMetadata
        let metadata: Vec<ComponentMetadata> = registrations
            .into_iter()
            .map(|reg| ComponentMetadata {
                type_info: infrastructure_common::TypeInfo {
                    id: reg.type_id,
                    name: reg.type_name.clone(),
                    module_path: "unknown".to_string(),
                },
                name: reg.type_name.split("::").last().unwrap_or(&reg.type_name).to_string(),
                description: Some("Auto-discovered component".to_string()),
                version: Some("1.0.0".to_string()),
                author: Some("System".to_string()),
                tags: reg.tags.into_iter().collect(),
                properties: std::collections::HashMap::new(),
            })
            .collect();
        
        Ok(metadata)
    }
    
    fn name(&self) -> &str {
        "ComponentScannerImpl"
    }
    
    fn supports(&self, target: &str) -> bool {
        // 支持扫描 Rust crate 和模块
        target.starts_with("crate::") || target.contains("::")
    }
}

/// 组件扫描器构建器
pub struct ComponentScannerBuilder {
    strategy: ComponentDiscoveryStrategy,
    packages: Vec<String>,
}

impl ComponentScannerBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            strategy: ComponentDiscoveryStrategy::Automatic,
            packages: Vec::new(),
        }
    }
    
    /// 设置发现策略
    pub fn with_strategy(mut self, strategy: ComponentDiscoveryStrategy) -> Self {
        self.strategy = strategy;
        self
    }
    
    /// 添加扫描包
    pub fn scan_package<S: Into<String>>(mut self, package: S) -> Self {
        self.packages.push(package.into());
        self
    }
    
    /// 自动发现常见包
    pub fn auto_discover_packages(mut self) -> Self {
        // 添加常见的包名模式
        let common_packages = [
            "crate::services",
            "crate::repositories",
            "crate::managers",
            "crate::handlers",
            "crate::components",
        ];
        
        for package in &common_packages {
            self.packages.push(package.to_string());
        }
        
        self
    }
    
    /// 构建组件扫描器
    pub fn build(self) -> ComponentScannerImpl {
        let mut scanner = ComponentScannerImpl::new(self.strategy);
        
        for package in self.packages {
            scanner = scanner.add_scan_package(package);
        }
        
        scanner
    }
}

impl Default for ComponentScannerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 高级组件管理器
/// 
/// 提供组件的高级管理功能，包括生命周期管理、健康检查等
pub struct AdvancedComponentManager {
    /// 组件扫描器
    scanner: ComponentScannerImpl,
    /// 注册的组件实例
    instances: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
    /// 组件状态
    component_states: Arc<RwLock<HashMap<TypeId, ComponentState>>>,
}

/// 组件状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentState {
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
    /// 失败
    Failed,
}

impl AdvancedComponentManager {
    /// 创建新的组件管理器
    pub fn new(scanner: ComponentScannerImpl) -> Self {
        Self {
            scanner,
            instances: Arc::new(RwLock::new(HashMap::new())),
            component_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 启动所有组件
    pub async fn start_all_components(&self) -> Result<(), DependencyError> {
        info!("开始启动所有组件");
        
        let components = self.scanner.get_discovered_components().await;
        let mut started_count = 0;
        
        for component in &components {
            if let Err(e) = self.start_component(component.type_id).await {
                error!("启动组件失败: {} - {:?}", component.type_name, e);
                return Err(e);
            }
            started_count += 1;
        }
        
        info!("所有组件启动完成，共启动 {} 个组件", started_count);
        Ok(())
    }
    
    /// 启动单个组件
    pub async fn start_component(&self, type_id: TypeId) -> Result<(), DependencyError> {
        let mut states = self.component_states.write().await;
        states.insert(type_id, ComponentState::Initializing);
        
        // TODO: 实际的组件启动逻辑
        
        states.insert(type_id, ComponentState::Running);
        debug!("组件启动成功: {:?}", type_id);
        
        Ok(())
    }
    
    /// 停止所有组件
    pub async fn stop_all_components(&self) -> Result<(), DependencyError> {
        info!("开始停止所有组件");
        
        let components = self.scanner.get_discovered_components().await;
        let mut stopped_count = 0;
        
        // 按依赖关系逆序停止组件
        for component in components.iter().rev() {
            if let Err(e) = self.stop_component(component.type_id).await {
                error!("停止组件失败: {} - {:?}", component.type_name, e);
                // 继续停止其他组件
            } else {
                stopped_count += 1;
            }
        }
        
        info!("所有组件停止完成，共停止 {} 个组件", stopped_count);
        Ok(())
    }
    
    /// 停止单个组件
    pub async fn stop_component(&self, type_id: TypeId) -> Result<(), DependencyError> {
        let mut states = self.component_states.write().await;
        states.insert(type_id, ComponentState::Stopping);
        
        // TODO: 实际的组件停止逻辑
        
        states.insert(type_id, ComponentState::Stopped);
        debug!("组件停止成功: {:?}", type_id);
        
        Ok(())
    }
    
    /// 获取组件状态
    pub async fn get_component_state(&self, type_id: TypeId) -> Option<ComponentState> {
        let states = self.component_states.read().await;
        states.get(&type_id).cloned()
    }
    
    /// 获取所有组件状态
    pub async fn get_all_component_states(&self) -> HashMap<TypeId, ComponentState> {
        let states = self.component_states.read().await;
        states.clone()
    }
}
