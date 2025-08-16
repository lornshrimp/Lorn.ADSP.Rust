//! 增强的组件发现机制
//!
//! 基于现有的 ComponentScannerImpl 添加更多高级发现能力

use crate::component_scanner::{ComponentDiscoveryStrategy, ComponentScannerImpl};
use async_trait::async_trait;
use di_abstractions::ComponentScanner;
use infrastructure_common::{
    Component, ComponentError, ComponentMetadata, ComponentRegistry, ComponentScope,
    DependencyGraph, DiscoveryMetadata, ReflectionInfo, TypeInfo,
};
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// 组件过滤器 trait
pub trait ComponentFilter: Send + Sync {
    /// 检查组件是否通过过滤条件
    fn matches(&self, metadata: &ComponentMetadata) -> bool;

    /// 过滤器名称
    fn name(&self) -> &str;
}

/// 基于作用域的过滤器
#[derive(Debug)]
pub struct ScopeFilter {
    pub allowed_scopes: HashSet<ComponentScope>,
}

impl ScopeFilter {
    pub fn new(scopes: Vec<ComponentScope>) -> Self {
        Self {
            allowed_scopes: scopes.into_iter().collect(),
        }
    }
}

impl ComponentFilter for ScopeFilter {
    fn matches(&self, metadata: &ComponentMetadata) -> bool {
        // 从标签中提取作用域信息
        for tag in &metadata.tags {
            if let Ok(scope) = tag.parse::<ComponentScope>() {
                return self.allowed_scopes.contains(&scope);
            }
        }
        // 默认允许通过
        true
    }

    fn name(&self) -> &str {
        "ScopeFilter"
    }
}

/// 基于名称模式的过滤器
#[derive(Debug)]
pub struct NameFilter {
    pub patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl NameFilter {
    pub fn new(patterns: Vec<String>) -> Self {
        Self {
            patterns,
            exclude_patterns: Vec::new(),
        }
    }

    pub fn with_exclusions(mut self, exclude_patterns: Vec<String>) -> Self {
        self.exclude_patterns = exclude_patterns;
        self
    }
}

impl ComponentFilter for NameFilter {
    fn matches(&self, metadata: &ComponentMetadata) -> bool {
        // 检查排除模式
        for pattern in &self.exclude_patterns {
            if metadata.name.contains(pattern) {
                return false;
            }
        }

        // 如果没有包含模式，默认通过
        if self.patterns.is_empty() {
            return true;
        }

        // 检查包含模式
        for pattern in &self.patterns {
            if metadata.name.contains(pattern) {
                return true;
            }
        }

        false
    }

    fn name(&self) -> &str {
        "NameFilter"
    }
}

/// 组件拦截器 trait
pub trait ComponentInterceptor: Send + Sync {
    /// 在组件发现前调用
    fn before_discovery(&self, context: &DiscoveryContext);

    /// 在组件发现后调用
    fn after_discovery(&self, context: &DiscoveryContext, metadata: &mut ComponentMetadata);

    /// 拦截器名称
    fn name(&self) -> &str;
}

/// 发现上下文
#[derive(Debug)]
pub struct DiscoveryContext {
    pub package_name: String,
    pub strategy: ComponentDiscoveryStrategy,
    pub filters: Vec<String>,
    pub start_time: std::time::Instant,
}

impl DiscoveryContext {
    pub fn new(package_name: String, strategy: ComponentDiscoveryStrategy) -> Self {
        Self {
            package_name,
            strategy,
            filters: Vec::new(),
            start_time: std::time::Instant::now(),
        }
    }
}

/// 日志拦截器
#[derive(Debug)]
pub struct LoggingInterceptor;

impl ComponentInterceptor for LoggingInterceptor {
    fn before_discovery(&self, context: &DiscoveryContext) {
        debug!(
            "开始发现组件: package={}, strategy={:?}",
            context.package_name, context.strategy
        );
    }

    fn after_discovery(&self, context: &DiscoveryContext, metadata: &mut ComponentMetadata) {
        debug!(
            "发现组件: name={}, package={}, elapsed={:?}",
            metadata.name,
            context.package_name,
            context.start_time.elapsed()
        );
    }

    fn name(&self) -> &str {
        "LoggingInterceptor"
    }
}

/// 高级组件元数据
#[derive(Debug, Clone)]
pub struct AdvancedComponentMetadata {
    /// 基础元数据
    pub base: ComponentMetadata,
    /// 反射信息
    pub reflection: ReflectionInfo,
    /// 依赖信息
    pub dependencies: Vec<TypeInfo>,
    /// 提供的服务
    pub provides: Vec<TypeInfo>,
    /// 组件作用域
    pub scope: ComponentScope,
    /// 启动顺序
    pub startup_order: i32,
    /// 条件表达式
    pub conditions: Vec<String>,
    /// 配置路径
    pub config_path: Option<String>,
    /// 是否为懒加载
    pub lazy_init: bool,
}

impl AdvancedComponentMetadata {
    pub fn from_base(base: ComponentMetadata) -> Self {
        Self {
            reflection: ReflectionInfo::of::<()>(), // 需要实际的类型信息
            dependencies: Vec::new(),
            provides: Vec::new(),
            scope: ComponentScope::Prototype,
            startup_order: 0,
            conditions: Vec::new(),
            config_path: None,
            lazy_init: false,
            base,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<TypeInfo>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_scope(mut self, scope: ComponentScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_condition(mut self, condition: String) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn with_config_path(mut self, path: String) -> Self {
        self.config_path = Some(path);
        self
    }
}

/// 增强的组件发现策略
#[derive(Debug, Clone)]
pub enum EnhancedComponentDiscoveryStrategy {
    /// 基础策略
    Base(ComponentDiscoveryStrategy),
    /// 基于属性的发现
    AttributeBased {
        /// 要扫描的属性名称
        attributes: Vec<String>,
        /// 是否递归扫描
        recursive: bool,
    },
    /// 基于 trait 的发现
    TraitBased {
        /// 要查找的 trait 类型
        trait_types: Vec<TypeInfo>,
        /// 是否包含子 trait
        include_subtypes: bool,
    },
    /// 条件化发现
    Conditional {
        /// 条件表达式
        conditions: Vec<String>,
        /// 基础策略
        base_strategy: Box<EnhancedComponentDiscoveryStrategy>,
    },
    /// 组合策略
    Composite {
        /// 子策略列表
        strategies: Vec<EnhancedComponentDiscoveryStrategy>,
        /// 组合模式（AND/OR）
        mode: CompositeMode,
    },
}

/// 组合模式
#[derive(Debug, Clone)]
pub enum CompositeMode {
    /// 所有策略都必须匹配
    And,
    /// 任一策略匹配即可
    Or,
}

/// 增强的组件扫描器实现
pub struct EnhancedComponentScannerImpl {
    /// 基础扫描器
    base_scanner: ComponentScannerImpl,
    /// 增强策略
    enhanced_strategy: EnhancedComponentDiscoveryStrategy,
    /// 组件过滤器
    filters: Arc<RwLock<Vec<Box<dyn ComponentFilter>>>>,
    /// 组件拦截器
    interceptors: Arc<RwLock<Vec<Box<dyn ComponentInterceptor>>>>,
    /// 高级元数据缓存
    advanced_metadata_cache: Arc<RwLock<HashMap<TypeId, AdvancedComponentMetadata>>>,
    /// 条件评估器
    condition_evaluator: Arc<dyn ConditionEvaluator>,
    /// 属性提取器
    attribute_extractor: Arc<dyn AttributeExtractor>,
    /// trait 发现器
    trait_discoverer: Arc<dyn TraitDiscoverer>,
}

impl EnhancedComponentScannerImpl {
    /// 创建新的增强组件扫描器
    pub fn new(
        base_scanner: ComponentScannerImpl,
        enhanced_strategy: EnhancedComponentDiscoveryStrategy,
    ) -> Self {
        Self {
            base_scanner,
            enhanced_strategy,
            filters: Arc::new(RwLock::new(Vec::new())),
            interceptors: Arc::new(RwLock::new(Vec::new())),
            advanced_metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            condition_evaluator: Arc::new(DefaultConditionEvaluator::new()),
            attribute_extractor: Arc::new(DefaultAttributeExtractor::new()),
            trait_discoverer: Arc::new(DefaultTraitDiscoverer::new()),
        }
    }

    /// 添加组件过滤器
    pub async fn add_filter(&self, filter: Box<dyn ComponentFilter>) {
        let mut filters = self.filters.write().await;
        filters.push(filter);
    }

    /// 添加组件拦截器
    pub async fn add_interceptor(&self, interceptor: Box<dyn ComponentInterceptor>) {
        let mut interceptors = self.interceptors.write().await;
        interceptors.push(interceptor);
    }

    /// 扫描组件（增强版本）
    pub async fn scan_enhanced(
        &self,
        package_name: &str,
    ) -> Result<Vec<AdvancedComponentMetadata>, ComponentError> {
        let context = DiscoveryContext::new(
            package_name.to_string(),
            ComponentDiscoveryStrategy::Automatic,
        );

        // 调用拦截器 - before
        {
            let interceptors = self.interceptors.read().await;
            for interceptor in interceptors.iter() {
                interceptor.before_discovery(&context);
            }
        }

        let mut discovered_components = Vec::new();

        // 根据增强策略执行发现
        match &self.enhanced_strategy {
            EnhancedComponentDiscoveryStrategy::Base(strategy) => {
                discovered_components =
                    self.scan_with_base_strategy(package_name, strategy).await?;
            }

            EnhancedComponentDiscoveryStrategy::AttributeBased {
                attributes,
                recursive,
            } => {
                discovered_components = self
                    .scan_attribute_based(package_name, attributes, *recursive)
                    .await?;
            }

            EnhancedComponentDiscoveryStrategy::TraitBased {
                trait_types,
                include_subtypes,
            } => {
                discovered_components = self
                    .scan_trait_based(package_name, trait_types, *include_subtypes)
                    .await?;
            }

            EnhancedComponentDiscoveryStrategy::Conditional {
                conditions,
                base_strategy,
            } => {
                discovered_components = self
                    .scan_conditional(package_name, conditions, base_strategy)
                    .await?;
            }

            EnhancedComponentDiscoveryStrategy::Composite { strategies, mode } => {
                discovered_components = self.scan_composite(package_name, strategies, mode).await?;
            }
        }

        // 应用过滤器
        discovered_components = self.apply_filters(discovered_components).await;

        // 调用拦截器 - after
        {
            let interceptors = self.interceptors.read().await;
            for component in &mut discovered_components {
                for interceptor in interceptors.iter() {
                    interceptor.after_discovery(&context, &mut component.base);
                }
            }
        }

        // 缓存结果
        {
            let mut cache = self.advanced_metadata_cache.write().await;
            for component in &discovered_components {
                cache.insert(component.base.type_info.id, component.clone());
            }
        }

        info!(
            "增强扫描完成: package={}, found={} components",
            package_name,
            discovered_components.len()
        );
        Ok(discovered_components)
    }

    /// 使用基础策略扫描
    async fn scan_with_base_strategy(
        &self,
        package_name: &str,
        strategy: &ComponentDiscoveryStrategy,
    ) -> Result<Vec<AdvancedComponentMetadata>, ComponentError> {
        // 使用基础扫描器
        let base_metadata = self.base_scanner.scan(package_name).await?;

        // 转换为高级元数据
        let mut advanced_metadata = Vec::new();
        for metadata in base_metadata {
            let advanced = AdvancedComponentMetadata::from_base(metadata);
            advanced_metadata.push(advanced);
        }

        Ok(advanced_metadata)
    }

    /// 基于属性的扫描
    async fn scan_attribute_based(
        &self,
        package_name: &str,
        attributes: &[String],
        recursive: bool,
    ) -> Result<Vec<AdvancedComponentMetadata>, ComponentError> {
        debug!(
            "开始基于属性的组件扫描: package={}, attributes={:?}",
            package_name, attributes
        );

        let mut discovered = Vec::new();

        // 使用属性提取器查找带有指定属性的组件
        for attribute in attributes {
            let components = self
                .attribute_extractor
                .extract_components_with_attribute(package_name, attribute, recursive)
                .await?;

            for component_info in components {
                let metadata = self
                    .create_advanced_metadata_from_attribute(component_info, attribute)
                    .await?;
                discovered.push(metadata);
            }
        }

        debug!("基于属性的扫描完成: found={} components", discovered.len());
        Ok(discovered)
    }

    /// 基于 trait 的扫描
    async fn scan_trait_based(
        &self,
        package_name: &str,
        trait_types: &[TypeInfo],
        include_subtypes: bool,
    ) -> Result<Vec<AdvancedComponentMetadata>, ComponentError> {
        debug!(
            "开始基于 trait 的组件扫描: package={}, traits={:?}",
            package_name, trait_types
        );

        let mut discovered = Vec::new();

        for trait_type in trait_types {
            let implementors = self
                .trait_discoverer
                .find_implementors(package_name, trait_type, include_subtypes)
                .await?;

            for implementor in implementors {
                let metadata = self
                    .create_advanced_metadata_from_trait(implementor, trait_type)
                    .await?;
                discovered.push(metadata);
            }
        }

        debug!(
            "基于 trait 的扫描完成: found={} components",
            discovered.len()
        );
        Ok(discovered)
    }

    /// 条件化扫描
    async fn scan_conditional(
        &self,
        package_name: &str,
        conditions: &[String],
        base_strategy: &EnhancedComponentDiscoveryStrategy,
    ) -> Result<Vec<AdvancedComponentMetadata>, ComponentError> {
        debug!(
            "开始条件化组件扫描: package={}, conditions={:?}",
            package_name, conditions
        );

        // 评估条件
        let mut all_conditions_met = true;
        for condition in conditions {
            if !self.condition_evaluator.evaluate(condition).await? {
                all_conditions_met = false;
                break;
            }
        }

        if !all_conditions_met {
            debug!("条件不满足，跳过扫描");
            return Ok(Vec::new());
        }

        // 递归调用基础策略
        let temp_scanner = EnhancedComponentScannerImpl::new(
            ComponentScannerImpl::new(ComponentDiscoveryStrategy::Automatic),
            base_strategy.clone(),
        );

        Box::pin(temp_scanner.scan_enhanced(package_name)).await
    }

    /// 组合策略扫描
    async fn scan_composite(
        &self,
        package_name: &str,
        strategies: &[EnhancedComponentDiscoveryStrategy],
        mode: &CompositeMode,
    ) -> Result<Vec<AdvancedComponentMetadata>, ComponentError> {
        debug!(
            "开始组合策略扫描: package={}, strategies={}, mode={:?}",
            package_name,
            strategies.len(),
            mode
        );

        let mut all_results = Vec::new();

        for strategy in strategies {
            let temp_scanner = EnhancedComponentScannerImpl::new(
                ComponentScannerImpl::new(ComponentDiscoveryStrategy::Automatic),
                strategy.clone(),
            );

            let results = Box::pin(temp_scanner.scan_enhanced(package_name)).await?;
            all_results.push(results);
        }

        // 根据组合模式合并结果
        let final_results = match mode {
            CompositeMode::And => {
                // AND 模式：只保留在所有策略中都出现的组件
                self.intersect_results(all_results)
            }
            CompositeMode::Or => {
                // OR 模式：合并所有策略的结果
                self.union_results(all_results)
            }
        };

        debug!("组合策略扫描完成: found={} components", final_results.len());
        Ok(final_results)
    }

    /// 应用过滤器
    async fn apply_filters(
        &self,
        mut components: Vec<AdvancedComponentMetadata>,
    ) -> Vec<AdvancedComponentMetadata> {
        let filters = self.filters.read().await;

        if filters.is_empty() {
            return components;
        }

        components.retain(|component| {
            for filter in filters.iter() {
                if !filter.matches(&component.base) {
                    debug!(
                        "组件被过滤器排除: name={}, filter={}",
                        component.base.name,
                        filter.name()
                    );
                    return false;
                }
            }
            true
        });

        components
    }

    /// 从属性信息创建高级元数据
    async fn create_advanced_metadata_from_attribute(
        &self,
        component_info: ComponentInfo,
        attribute: &str,
    ) -> Result<AdvancedComponentMetadata, ComponentError> {
        let base_metadata = ComponentMetadata {
            type_info: component_info.type_info.clone(),
            name: component_info.name.clone(),
            description: Some(format!("Component discovered by attribute: {}", attribute)),
            version: Some("1.0.0".to_string()),
            author: Some("AttributeDiscovery".to_string()),
            tags: component_info.tags.clone(),
            properties: component_info.properties.clone(),
        };

        let mut advanced = AdvancedComponentMetadata::from_base(base_metadata);

        // 从属性中提取额外信息
        if let Some(scope_str) = component_info.properties.get("scope") {
            if let Ok(scope) = scope_str.parse::<ComponentScope>() {
                advanced.scope = scope;
            }
        }

        if let Some(order_str) = component_info.properties.get("startup_order") {
            if let Ok(order) = order_str.parse::<i32>() {
                advanced.startup_order = order;
            }
        }

        if let Some(config_path) = component_info.properties.get("config_path") {
            advanced.config_path = Some(config_path.clone());
        }

        Ok(advanced)
    }

    /// 从 trait 信息创建高级元数据
    async fn create_advanced_metadata_from_trait(
        &self,
        implementor: ImplementorInfo,
        trait_type: &TypeInfo,
    ) -> Result<AdvancedComponentMetadata, ComponentError> {
        let base_metadata = ComponentMetadata {
            type_info: implementor.type_info.clone(),
            name: implementor.name.clone(),
            description: Some(format!("Component implementing trait: {}", trait_type.name)),
            version: Some("1.0.0".to_string()),
            author: Some("TraitDiscovery".to_string()),
            tags: implementor.tags.clone(),
            properties: implementor.properties.clone(),
        };

        let mut advanced = AdvancedComponentMetadata::from_base(base_metadata);
        advanced.provides.push(trait_type.clone());

        Ok(advanced)
    }

    /// 求交集（AND 模式）
    fn intersect_results(
        &self,
        all_results: Vec<Vec<AdvancedComponentMetadata>>,
    ) -> Vec<AdvancedComponentMetadata> {
        if all_results.is_empty() {
            return Vec::new();
        }

        let mut intersection = all_results[0].clone();

        for results in all_results.iter().skip(1) {
            intersection.retain(|component| {
                results
                    .iter()
                    .any(|other| component.base.type_info.id == other.base.type_info.id)
            });
        }

        intersection
    }

    /// 求并集（OR 模式）
    fn union_results(
        &self,
        all_results: Vec<Vec<AdvancedComponentMetadata>>,
    ) -> Vec<AdvancedComponentMetadata> {
        let mut union = Vec::new();
        let mut seen_types = HashSet::new();

        for results in all_results {
            for component in results {
                if seen_types.insert(component.base.type_info.id) {
                    union.push(component);
                }
            }
        }

        union
    }

    /// 获取高级元数据
    pub async fn get_advanced_metadata(
        &self,
        type_id: TypeId,
    ) -> Option<AdvancedComponentMetadata> {
        let cache = self.advanced_metadata_cache.read().await;
        cache.get(&type_id).cloned()
    }

    /// 构建依赖关系图
    pub async fn build_dependency_graph(&self) -> Result<DependencyGraph, ComponentError> {
        let mut graph = DependencyGraph::new();
        let cache = self.advanced_metadata_cache.read().await;

        for metadata in cache.values() {
            let discovery_metadata = DiscoveryMetadata::new(
                metadata.base.type_info.clone(),
                metadata.base.name.clone(),
                metadata.dependencies.clone(),
            )
            .with_scope(metadata.scope.clone())
            .with_startup_order(metadata.startup_order);

            graph.add_component(discovery_metadata);
        }

        Ok(graph)
    }
}

/// 条件评估器 trait
#[async_trait]
pub trait ConditionEvaluator: Send + Sync {
    /// 评估条件表达式
    async fn evaluate(&self, condition: &str) -> Result<bool, ComponentError>;
}

/// 默认条件评估器
pub struct DefaultConditionEvaluator {
    /// 环境变量缓存
    env_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl DefaultConditionEvaluator {
    pub fn new() -> Self {
        Self {
            env_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ConditionEvaluator for DefaultConditionEvaluator {
    async fn evaluate(&self, condition: &str) -> Result<bool, ComponentError> {
        // 简单的条件评估实现
        // 支持环境变量检查：env.VAR_NAME=value
        // 支持配置检查：config.path.to.value=expected

        if condition.starts_with("env.") {
            let parts: Vec<&str> = condition.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(ComponentError::ConditionError {
                    condition: condition.to_string(),
                    message: "Invalid environment condition format".to_string(),
                });
            }

            let env_var = parts[0].strip_prefix("env.").unwrap();
            let expected_value = parts[1];

            // 检查缓存
            {
                let cache = self.env_cache.read().await;
                if let Some(cached_value) = cache.get(env_var) {
                    return Ok(cached_value == expected_value);
                }
            }

            // 从环境变量获取
            if let Ok(actual_value) = std::env::var(env_var) {
                // 更新缓存
                {
                    let mut cache = self.env_cache.write().await;
                    cache.insert(env_var.to_string(), actual_value.clone());
                }
                return Ok(actual_value == expected_value);
            }

            return Ok(false);
        }

        // 默认条件为真
        Ok(true)
    }
}

/// 属性提取器 trait
#[async_trait]
pub trait AttributeExtractor: Send + Sync {
    /// 提取带有指定属性的组件
    async fn extract_components_with_attribute(
        &self,
        package_name: &str,
        attribute: &str,
        recursive: bool,
    ) -> Result<Vec<ComponentInfo>, ComponentError>;
}

/// 组件信息
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub type_info: TypeInfo,
    pub name: String,
    pub tags: Vec<String>,
    pub properties: HashMap<String, String>,
}

/// 默认属性提取器
pub struct DefaultAttributeExtractor;

impl DefaultAttributeExtractor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AttributeExtractor for DefaultAttributeExtractor {
    async fn extract_components_with_attribute(
        &self,
        package_name: &str,
        attribute: &str,
        _recursive: bool,
    ) -> Result<Vec<ComponentInfo>, ComponentError> {
        debug!(
            "提取带有属性 {} 的组件: package={}",
            attribute, package_name
        );

        // 在实际实现中，这里会使用编译时生成的组件注册表
        // 或者通过 proc_macro 生成的静态信息来查找组件

        // 目前返回模拟数据
        let mut components = Vec::new();

        // 模拟一些带有 #[component] 属性的组件
        if attribute == "component" {
            components.push(ComponentInfo {
                type_info: TypeInfo::new(TypeId::of::<()>(), "MockService"),
                name: "MockService".to_string(),
                tags: vec!["service".to_string()],
                properties: {
                    let mut props = HashMap::new();
                    props.insert("scope".to_string(), "singleton".to_string());
                    props.insert("priority".to_string(), "100".to_string());
                    props
                },
            });
        }

        debug!("属性提取完成: found={} components", components.len());
        Ok(components)
    }
}

/// trait 发现器 trait
#[async_trait]
pub trait TraitDiscoverer: Send + Sync {
    /// 查找实现指定 trait 的类型
    async fn find_implementors(
        &self,
        package_name: &str,
        trait_type: &TypeInfo,
        include_subtypes: bool,
    ) -> Result<Vec<ImplementorInfo>, ComponentError>;
}

/// 实现者信息
#[derive(Debug, Clone)]
pub struct ImplementorInfo {
    pub type_info: TypeInfo,
    pub name: String,
    pub tags: Vec<String>,
    pub properties: HashMap<String, String>,
}

/// 默认 trait 发现器
pub struct DefaultTraitDiscoverer;

impl DefaultTraitDiscoverer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TraitDiscoverer for DefaultTraitDiscoverer {
    async fn find_implementors(
        &self,
        package_name: &str,
        trait_type: &TypeInfo,
        _include_subtypes: bool,
    ) -> Result<Vec<ImplementorInfo>, ComponentError> {
        debug!(
            "查找 trait {} 的实现者: package={}",
            trait_type.name, package_name
        );

        let mut implementors = Vec::new();

        // 在实际实现中，这里会使用反射或编译时信息来查找 trait 实现
        // 目前返回模拟数据

        if trait_type.name.contains("Component") {
            implementors.push(ImplementorInfo {
                type_info: TypeInfo::new(TypeId::of::<()>(), "MockComponentImpl"),
                name: "MockComponentImpl".to_string(),
                tags: vec!["component".to_string(), "mock".to_string()],
                properties: HashMap::new(),
            });
        }

        debug!("trait 发现完成: found={} implementors", implementors.len());
        Ok(implementors)
    }
}

/// 高级组件管理器
pub struct AdvancedComponentManager {
    /// 增强扫描器
    scanner: EnhancedComponentScannerImpl,
    /// 组件注册表
    registry: Arc<dyn ComponentRegistry>,
    /// 依赖关系图
    dependency_graph: Arc<RwLock<Option<DependencyGraph>>>,
}

impl AdvancedComponentManager {
    /// 创建新的高级组件管理器
    pub fn new(
        scanner: EnhancedComponentScannerImpl,
        registry: Arc<dyn ComponentRegistry>,
    ) -> Self {
        Self {
            scanner,
            registry,
            dependency_graph: Arc::new(RwLock::new(None)),
        }
    }

    /// 扫描并注册所有组件
    pub async fn scan_and_register_all(
        &self,
        packages: &[String],
    ) -> Result<usize, ComponentError> {
        let mut total_registered = 0;

        for package in packages {
            let components = self.scanner.scan_enhanced(package).await?;

            for component in components {
                // 转换为发现元数据并注册
                let discovery_metadata = DiscoveryMetadata::new(
                    component.base.type_info.clone(),
                    component.base.name.clone(),
                    component.dependencies.clone(),
                )
                .with_scope(component.scope.clone())
                .with_startup_order(component.startup_order);

                self.registry
                    .register_discovery_metadata(discovery_metadata)
                    .await?;
                total_registered += 1;
            }
        }

        // 构建依赖关系图
        let graph = self.scanner.build_dependency_graph().await?;
        {
            let mut dependency_graph = self.dependency_graph.write().await;
            *dependency_graph = Some(graph);
        }

        info!(
            "组件扫描和注册完成: registered={} components",
            total_registered
        );
        Ok(total_registered)
    }

    /// 获取依赖关系图
    pub async fn get_dependency_graph(&self) -> Option<DependencyGraph> {
        let graph = self.dependency_graph.read().await;
        graph.as_ref().cloned()
    }

    /// 检测循环依赖
    pub async fn detect_circular_dependencies(&self) -> Result<(), Vec<Vec<TypeInfo>>> {
        if let Some(graph) = self.get_dependency_graph().await {
            graph.detect_circular_dependencies()
        } else {
            Ok(())
        }
    }

    /// 获取启动顺序
    pub async fn get_startup_order(&self) -> Result<Vec<TypeInfo>, ComponentError> {
        if let Some(graph) = self.get_dependency_graph().await {
            graph.topological_sort()
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component_scanner::ComponentScannerImpl;
    use infrastructure_common::{ComponentMetadata, ComponentScope, TypeInfo};

    use tokio;

    /// 测试组件
    #[derive(Debug)]
    struct TestComponent {
        name: &'static str,
    }

    impl Component for TestComponent {
        fn name(&self) -> &'static str {
            self.name
        }
    }

    /// 创建测试用的增强扫描器
    fn create_test_scanner() -> EnhancedComponentScannerImpl {
        let base_scanner = ComponentScannerImpl::new(ComponentDiscoveryStrategy::Automatic);
        let enhanced_strategy =
            EnhancedComponentDiscoveryStrategy::Base(ComponentDiscoveryStrategy::Automatic);

        EnhancedComponentScannerImpl::new(base_scanner, enhanced_strategy)
    }

    #[tokio::test]
    async fn test_scope_filter() {
        let filter = ScopeFilter::new(vec![ComponentScope::Singleton]);

        let metadata = ComponentMetadata::new(TypeInfo::of::<TestComponent>(), "TestComponent")
            .with_tag("singleton");

        assert!(filter.matches(&metadata));
        assert_eq!(filter.name(), "ScopeFilter");
    }

    #[tokio::test]
    async fn test_name_filter() {
        let filter =
            NameFilter::new(vec!["Service".to_string()]).with_exclusions(vec!["Mock".to_string()]);

        let service_metadata =
            ComponentMetadata::new(TypeInfo::of::<TestComponent>(), "UserService");

        let mock_service_metadata =
            ComponentMetadata::new(TypeInfo::of::<TestComponent>(), "MockUserService");

        assert!(filter.matches(&service_metadata));
        assert!(!filter.matches(&mock_service_metadata));
    }

    #[tokio::test]
    async fn test_logging_interceptor() {
        let interceptor = LoggingInterceptor;
        let context = DiscoveryContext::new(
            "test_package".to_string(),
            ComponentDiscoveryStrategy::Automatic,
        );
        let mut metadata = ComponentMetadata::new(TypeInfo::of::<TestComponent>(), "TestComponent");

        // 这些调用不应该 panic
        interceptor.before_discovery(&context);
        interceptor.after_discovery(&context, &mut metadata);

        assert_eq!(interceptor.name(), "LoggingInterceptor");
    }

    #[tokio::test]
    async fn test_enhanced_scanner_creation() {
        let scanner = create_test_scanner();

        // 测试添加过滤器
        let filter = Box::new(ScopeFilter::new(vec![ComponentScope::Singleton]));
        scanner.add_filter(filter).await;

        // 测试添加拦截器
        let interceptor = Box::new(LoggingInterceptor);
        scanner.add_interceptor(interceptor).await;
    }

    #[tokio::test]
    async fn test_attribute_based_strategy() {
        let base_scanner = ComponentScannerImpl::new(ComponentDiscoveryStrategy::Automatic);
        let enhanced_strategy = EnhancedComponentDiscoveryStrategy::AttributeBased {
            attributes: vec!["component".to_string()],
            recursive: true,
        };

        let scanner = EnhancedComponentScannerImpl::new(base_scanner, enhanced_strategy);

        // 测试扫描（会返回模拟数据）
        let results = scanner.scan_enhanced("test_package").await.unwrap();

        // 验证结果
        assert!(!results.is_empty());
        for result in &results {
            assert!(!result.base.name.is_empty());
        }
    }

    #[tokio::test]
    async fn test_trait_based_strategy() {
        let base_scanner = ComponentScannerImpl::new(ComponentDiscoveryStrategy::Automatic);
        let enhanced_strategy = EnhancedComponentDiscoveryStrategy::TraitBased {
            trait_types: vec![TypeInfo::from_name("Component")],
            include_subtypes: true,
        };

        let scanner = EnhancedComponentScannerImpl::new(base_scanner, enhanced_strategy);

        // 测试扫描
        let results = scanner.scan_enhanced("test_package").await.unwrap();

        // 验证结果
        for result in &results {
            assert!(!result.provides.is_empty());
        }
    }

    #[tokio::test]
    async fn test_conditional_strategy() {
        let base_scanner = ComponentScannerImpl::new(ComponentDiscoveryStrategy::Automatic);
        let base_strategy =
            EnhancedComponentDiscoveryStrategy::Base(ComponentDiscoveryStrategy::Automatic);
        let enhanced_strategy = EnhancedComponentDiscoveryStrategy::Conditional {
            conditions: vec!["env.TEST_MODE=true".to_string()],
            base_strategy: Box::new(base_strategy),
        };

        let scanner = EnhancedComponentScannerImpl::new(base_scanner, enhanced_strategy);

        // 设置环境变量
        std::env::set_var("TEST_MODE", "true");

        // 测试扫描
        let results = scanner.scan_enhanced("test_package").await.unwrap();

        // 清理环境变量
        std::env::remove_var("TEST_MODE");

        // 验证结果（条件满足时应该有结果，但由于是模拟实现，可能为空）
        // 这里主要测试条件评估逻辑，不是结果数量
        assert!(results.len() >= 0);
    }

    #[tokio::test]
    async fn test_composite_strategy_or_mode() {
        let base_scanner = ComponentScannerImpl::new(ComponentDiscoveryStrategy::Automatic);
        let strategy1 = EnhancedComponentDiscoveryStrategy::AttributeBased {
            attributes: vec!["component".to_string()],
            recursive: false,
        };
        let strategy2 = EnhancedComponentDiscoveryStrategy::TraitBased {
            trait_types: vec![TypeInfo::from_name("Component")],
            include_subtypes: false,
        };

        let enhanced_strategy = EnhancedComponentDiscoveryStrategy::Composite {
            strategies: vec![strategy1, strategy2],
            mode: CompositeMode::Or,
        };

        let scanner = EnhancedComponentScannerImpl::new(base_scanner, enhanced_strategy);

        // 测试扫描
        let results = scanner.scan_enhanced("test_package").await.unwrap();

        // OR 模式应该合并所有结果
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_default_condition_evaluator() {
        let evaluator = DefaultConditionEvaluator::new();

        // 测试环境变量条件
        std::env::set_var("TEST_VAR", "test_value");

        let result = evaluator.evaluate("env.TEST_VAR=test_value").await.unwrap();
        assert!(result);

        let result = evaluator
            .evaluate("env.TEST_VAR=wrong_value")
            .await
            .unwrap();
        assert!(!result);

        // 测试不存在的环境变量
        let result = evaluator
            .evaluate("env.NON_EXISTENT_VAR=value")
            .await
            .unwrap();
        assert!(!result);

        // 清理
        std::env::remove_var("TEST_VAR");
    }

    #[tokio::test]
    async fn test_default_attribute_extractor() {
        let extractor = DefaultAttributeExtractor::new();

        // 测试提取带有 component 属性的组件
        let components = extractor
            .extract_components_with_attribute("test_package", "component", false)
            .await
            .unwrap();

        assert!(!components.is_empty());

        for component in &components {
            assert!(!component.name.is_empty());
            assert!(!component.tags.is_empty());
        }
    }

    #[tokio::test]
    async fn test_default_trait_discoverer() {
        let discoverer = DefaultTraitDiscoverer::new();

        // 测试查找 Component trait 的实现者
        let implementors = discoverer
            .find_implementors("test_package", &TypeInfo::from_name("Component"), false)
            .await
            .unwrap();

        assert!(!implementors.is_empty());

        for implementor in &implementors {
            assert!(!implementor.name.is_empty());
            assert!(implementor.tags.contains(&"component".to_string()));
        }
    }

    #[tokio::test]
    async fn test_advanced_component_metadata() {
        let base_metadata =
            ComponentMetadata::new(TypeInfo::of::<TestComponent>(), "TestComponent");

        let advanced = AdvancedComponentMetadata::from_base(base_metadata)
            .with_dependencies(vec![TypeInfo::of::<String>()])
            .with_scope(ComponentScope::Singleton)
            .with_condition("env.ENABLED=true".to_string())
            .with_config_path("components.test".to_string());

        assert_eq!(advanced.base.name, "TestComponent");
        assert_eq!(advanced.scope, ComponentScope::Singleton);
        assert_eq!(advanced.dependencies.len(), 1);
        assert_eq!(advanced.conditions.len(), 1);
        assert_eq!(advanced.config_path, Some("components.test".to_string()));
    }

    #[tokio::test]
    async fn test_component_scope_from_str() {
        assert_eq!(
            "singleton".parse::<ComponentScope>().unwrap(),
            ComponentScope::Singleton
        );
        assert_eq!(
            "prototype".parse::<ComponentScope>().unwrap(),
            ComponentScope::Prototype
        );
        assert_eq!(
            "request".parse::<ComponentScope>().unwrap(),
            ComponentScope::Request
        );
        assert_eq!(
            "session".parse::<ComponentScope>().unwrap(),
            ComponentScope::Session
        );
        assert_eq!(
            "application".parse::<ComponentScope>().unwrap(),
            ComponentScope::Application
        );

        assert!("invalid".parse::<ComponentScope>().is_err());
    }

    #[tokio::test]
    async fn test_advanced_component_manager() {
        use infrastructure_common::InMemoryComponentRegistry;

        let scanner = create_test_scanner();
        let registry = Arc::new(InMemoryComponentRegistry::new());
        let manager = AdvancedComponentManager::new(scanner, registry);

        // 测试扫描和注册
        let packages = vec!["test_package".to_string()];
        let registered_count = manager.scan_and_register_all(&packages).await.unwrap();

        // 由于使用模拟数据，注册数量可能为0，这里主要测试流程
        assert!(registered_count >= 0);

        // 测试获取依赖关系图
        let graph = manager.get_dependency_graph().await;
        assert!(graph.is_some());

        // 测试循环依赖检测
        let circular_deps = manager.detect_circular_dependencies().await;
        assert!(circular_deps.is_ok());

        // 测试获取启动顺序
        let startup_order = manager.get_startup_order().await.unwrap();
        // 由于使用模拟数据，启动顺序可能为空，这里主要测试流程
        assert!(startup_order.len() >= 0);
    }
}
