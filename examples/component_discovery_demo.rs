//! # 组件发现机制示例
//!
//! 演示完善的组件发现机制，包括：
//! - 使用过程宏实现编译时组件注册
//! - 实现基于反射的组件发现  
//! - 添加组件依赖关系分析

use std::any::TypeId;
use component_macros::{component, inject, service_provider, Component};
use infrastructure_common::{
    ComponentError, ComponentMetadata, TypeInfo,
    discovery::{
        ComponentDiscovery, ReflectionComponentDiscovery, ComponentScope, DiscoveryMetadata,
        DependencyGraph, ComponentRegistry,
    },
};
use infrastructure_composition::enhanced_component_scanner::{
    ComponentScannerImpl, ComponentFilter, ComponentInterceptor, LoggingInterceptor,
    ScopeFilter, NameFilter,
};
use di_abstractions::ComponentScanner;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

// ========== 示例业务组件 ==========

/// 用户仓储接口
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Option<User>;
    async fn save(&self, user: &User) -> Result<(), String>;
}

/// 用户实体
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

/// 用户仓储实现 - 使用编译时组件注册
#[component]
pub struct UserRepositoryImpl {
    // 这里可以有数据库连接等依赖
}

impl UserRepositoryImpl {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn find_by_id(&self, id: u64) -> Option<User> {
        // 模拟查询用户
        Some(User {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        })
    }

    async fn save(&self, _user: &User) -> Result<(), String> {
        // 模拟保存用户
        Ok(())
    }
}

/// 用户服务 - 使用依赖注入
#[component]
pub struct UserService {
    #[inject]
    user_repository: Arc<dyn UserRepository>,
}

impl UserService {
    pub fn new(user_repository: Arc<dyn UserRepository>) -> Self {
        Self { user_repository }
    }

    pub async fn get_user(&self, id: u64) -> Option<User> {
        self.user_repository.find_by_id(id).await
    }

    pub async fn create_user(&self, name: String, email: String) -> Result<User, String> {
        let user = User {
            id: rand::random(),
            name,
            email,
        };
        self.user_repository.save(&user).await?;
        Ok(user)
    }
}

/// 用户控制器 - 使用自动发现
#[derive(Component)]
pub struct UserController {
    user_service: Arc<UserService>,
}

impl UserController {
    pub fn new(user_service: Arc<UserService>) -> Self {
        Self { user_service }
    }

    pub async fn handle_get_user(&self, id: u64) -> Option<User> {
        self.user_service.get_user(id).await
    }
}

// ========== 服务提供者示例 ==========

/// 配置服务提供者
#[service_provider]
impl UserService {
    /// 创建用户服务工厂方法
    pub fn create_service(user_repository: Arc<dyn UserRepository>) -> Arc<UserService> {
        Arc::new(UserService::new(user_repository))
    }
}

// ========== 组件发现和依赖分析示例 ==========

/// 示例组件过滤器 - 只允许用户相关的组件
pub struct UserComponentFilter;

#[async_trait]
impl ComponentFilter for UserComponentFilter {
    async fn filter(&self, metadata: &ComponentMetadata) -> Result<bool, ComponentError> {
        // 过滤条件：组件名称包含 "User"
        Ok(metadata.name.contains("User"))
    }
}

/// 性能监控拦截器
pub struct PerformanceInterceptor;

#[async_trait]
impl ComponentInterceptor for PerformanceInterceptor {
    async fn before_registration(&self, metadata: &[ComponentMetadata]) -> Result<(), ComponentError> {
        info!("性能监控: 准备注册 {} 个组件", metadata.len());
        let start = std::time::Instant::now();
        // 记录开始时间
        Ok(())
    }

    async fn after_registration(&self, metadata: &[ComponentMetadata]) -> Result<(), ComponentError> {
        info!("性能监控: 完成注册 {} 个组件", metadata.len());
        // 可以记录注册耗时
        Ok(())
    }
}

/// 主要的组件发现示例函数
pub async fn demonstrate_component_discovery() -> Result<(), ComponentError> {
    info!("=== 组件发现机制演示 ===");

    // 1. 创建增强的组件扫描器
    let mut scanner = ComponentScannerImpl::new();
    
    // 2. 添加组件过滤器
    scanner.add_filter(Box::new(UserComponentFilter));
    scanner.add_filter(Box::new(NameFilter::new(vec!["Service".to_string(), "Repository".to_string()])));
    scanner.add_filter(Box::new(ScopeFilter::new(vec![ComponentScope::Singleton, ComponentScope::Transient])));
    
    // 3. 添加组件拦截器
    scanner.add_interceptor(Box::new(LoggingInterceptor));
    scanner.add_interceptor(Box::new(PerformanceInterceptor));
    
    info!("📦 开始扫描组件...");
    
    // 4. 执行组件扫描
    let discovered_components = scanner.scan("crate::user_module").await?;
    
    info!("🔍 发现了 {} 个组件:", discovered_components.len());
    for component in &discovered_components {
        info!("  - {}: {}", component.type_info.name, component.name);
    }
    
    // 5. 演示基于反射的组件发现
    info!("\n=== 基于反射的组件发现 ===");
    let reflection_discovery = ReflectionComponentDiscovery::new();
    let reflection_components = reflection_discovery.discover_components().await?;
    
    info!("🔍 反射发现了 {} 个组件:", reflection_components.len());
    for component in &reflection_components {
        info!("  - {}: {:?}", component.name, component.scope);
    }
    
    // 6. 演示依赖关系分析
    info!("\n=== 组件依赖关系分析 ===");
    demonstrate_dependency_analysis().await?;
    
    // 7. 演示全局组件注册表
    info!("\n=== 全局组件注册表 ===");
    demonstrate_global_registry().await?;
    
    info!("✅ 组件发现机制演示完成!");
    Ok(())
}

/// 演示依赖关系分析
async fn demonstrate_dependency_analysis() -> Result<(), ComponentError> {
    let mut dependency_graph = DependencyGraph::new();
    
    // 添加组件及其依赖关系
    let user_repo_type = TypeInfo::of::<UserRepositoryImpl>();
    let user_service_type = TypeInfo::of::<UserService>();
    let user_controller_type = TypeInfo::of::<UserController>();
    
    // 定义依赖关系
    dependency_graph.add_dependency(user_service_type.clone(), user_repo_type.clone())?;
    dependency_graph.add_dependency(user_controller_type.clone(), user_service_type.clone())?;
    
    info!("📊 依赖关系图:");
    info!("  UserController -> UserService");
    info!("  UserService -> UserRepositoryImpl");
    
    // 检测循环依赖
    match dependency_graph.detect_cycles() {
        Ok(_) => info!("✅ 没有检测到循环依赖"),
        Err(cycles) => {
            warn!("⚠️ 检测到循环依赖:");
            for cycle in cycles {
                warn!("  循环: {:?}", cycle);
            }
        }
    }
    
    // 获取拓扑排序（启动顺序）
    let startup_order = dependency_graph.topological_sort()?;
    info!("🚀 建议的组件启动顺序:");
    for (index, type_info) in startup_order.iter().enumerate() {
        info!("  {}. {}", index + 1, type_info.name);
    }
    
    Ok(())
}

/// 演示全局组件注册表
async fn demonstrate_global_registry() -> Result<(), ComponentError> {
    let registry = ComponentRegistry::new();
    
    // 模拟注册一些组件
    let user_repo = TypeInfo::of::<UserRepositoryImpl>();
    let user_service = TypeInfo::of::<UserService>();
    
    info!("📋 模拟组件注册:");
    info!("  注册 UserRepositoryImpl");
    info!("  注册 UserService");
    
    // 在实际应用中，这些会通过宏自动注册
    info!("✨ 编译时组件注册通过 #[component] 宏自动完成");
    
    Ok(())
}

/// 命令行工具示例
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 运行组件发现演示
    demonstrate_component_discovery().await?;
    
    Ok(())
}

// ========== 集成测试示例 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_component_discovery() {
        let mut scanner = ComponentScannerImpl::new();
        
        // 添加测试专用的过滤器
        scanner.add_filter(Box::new(NameFilter::new(vec!["Test".to_string()])));
        
        let result = scanner.scan("test_module").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        let type_a = TypeInfo::of::<UserService>();
        let type_b = TypeInfo::of::<UserRepositoryImpl>();
        
        assert!(graph.add_dependency(type_a, type_b).is_ok());
        assert!(graph.detect_cycles().is_ok());
    }

    #[test]
    fn test_component_macros() {
        // 测试编译时宏是否正常工作
        let _repo = UserRepositoryImpl::new();
        // 在实际测试中，可以验证宏生成的注册代码
    }
}
