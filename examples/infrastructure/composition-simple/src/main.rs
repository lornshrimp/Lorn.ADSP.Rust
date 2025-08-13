//! 组件发现机制简化示例（已迁移到工作区 examples）
//!
//! 演示如何使用反射机制进行组件注册和发现

use async_trait::async_trait;
use infrastructure_common::{
    get_global_component_registry, lifecycle::Lifetime, set_global_component_registry, Component,
    ComponentDescriptor, GlobalComponentRegistry,
};
use infrastructure_composition::ComponentScannerBuilder;
use std::sync::Arc;

// 示例：用户服务
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    cache: Arc<MockCacheService>,
}

// 示例：Repository trait
#[async_trait]
pub trait UserRepository: Send + Sync + std::fmt::Debug {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>>;
    async fn save(&self, user: &User) -> Result<(), Box<dyn std::error::Error>>;
}

// 示例：用户实体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl UserService {
    pub fn new(repository: Arc<dyn UserRepository>, cache: Arc<MockCacheService>) -> Self {
        Self { repository, cache }
    }

    pub async fn get_user(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>> {
        // 先尝试从缓存获取
        let cache_key = format!("user:{}", id);
        if let Ok(Some(user)) = self.cache.get_user(&cache_key).await {
            return Ok(Some(user));
        }

        // 从数据库获取
        if let Some(user) = self.repository.find_by_id(id).await? {
            // 缓存结果
            let _ = self.cache.set_user(&cache_key, &user).await;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
}

// 实现 Component trait
impl std::fmt::Debug for UserService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserService")
            .field("repository", &"<repository>")
            .field("cache", &self.cache)
            .finish()
    }
}

impl Component for UserService {
    fn name(&self) -> &'static str {
        "user_service"
    }
}

// 示例：模拟 Repository 实现
#[derive(Debug)]
pub struct MockUserRepository {
    users: std::collections::HashMap<u64, User>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        let mut users = std::collections::HashMap::new();
        users.insert(
            1,
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
        );
        users.insert(
            2,
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        );
        Self { users }
    }
}

impl Component for MockUserRepository {
    fn name(&self) -> &'static str {
        "mock_user_repository"
    }
}

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>> {
        Ok(self.users.get(&id).cloned())
    }

    async fn save(&self, _user: &User) -> Result<(), Box<dyn std::error::Error>> {
        // 模拟保存
        Ok(())
    }
}

// 示例：模拟缓存服务实现
#[derive(Debug)]
pub struct MockCacheService {
    cache: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl MockCacheService {
    pub fn new() -> Self {
        Self {
            cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Component for MockCacheService {
    fn name(&self) -> &'static str {
        "mock_cache_service"
    }
}

// 缓存抽象：示例最小化用户缓存接口（正式版本应迁移到 caching-abstractions crate）
#[async_trait]
pub trait UserCache: Send + Sync + std::fmt::Debug {
    async fn get_user(&self, key: &str) -> Result<Option<User>, Box<dyn std::error::Error>>;
    async fn set_user(&self, key: &str, user: &User) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
impl UserCache for MockCacheService {
    async fn get_user(&self, key: &str) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let guard = self.cache.lock().unwrap();
        Ok(guard.get(key).and_then(|v| serde_json::from_str(v).ok()))
    }
    async fn set_user(&self, key: &str, user: &User) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.cache.lock().unwrap();
        guard.insert(key.to_string(), serde_json::to_string(user)?);
        Ok(())
    }
}

/// 组件发现和注册示例
pub async fn component_discovery_example() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // 创建组件扫描器
    let _scanner = ComponentScannerBuilder::new().build();

    println!("组件发现示例启动成功");

    Ok(())
}

/// 手动注册组件示例
pub async fn manual_component_registration() -> Result<(), Box<dyn std::error::Error>> {
    // 若尚未设置全局注册表，这里设置一个简单内存实现
    if get_global_component_registry().is_none() {
        struct SimpleRegistry(std::sync::Mutex<Vec<ComponentDescriptor>>);
        impl GlobalComponentRegistry for SimpleRegistry {
            fn register_component_descriptor(
                &self,
                descriptor: ComponentDescriptor,
            ) -> Result<(), infrastructure_common::errors::InfrastructureError> {
                self.0.lock().unwrap().push(descriptor);
                Ok(())
            }
            fn get_all_descriptors(&self) -> Vec<ComponentDescriptor> {
                self.0.lock().unwrap().clone()
            }
        }
        set_global_component_registry(Arc::new(SimpleRegistry(std::sync::Mutex::new(Vec::new()))));
    }
    let registry = get_global_component_registry().unwrap();

    // 手动注册两个组件描述符（简化版本）
    registry.register_component_descriptor(
        ComponentDescriptor::new::<MockUserRepository>("mock_user_repository", Lifetime::Singleton)
            .with_priority(5)
            .with_metadata("layer", "infrastructure"),
    )?;
    registry.register_component_descriptor(
        ComponentDescriptor::new::<MockCacheService>("mock_cache_service", Lifetime::Singleton)
            .with_priority(4)
            .with_metadata("layer", "infrastructure"),
    )?;

    let all = registry.get_all_descriptors();
    println!("手动注册组件完成, 当前计数: {}", all.len());
    for d in all {
        println!("  - {} (prio={})", d.name, d.priority);
    }
    Ok(())
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 组件发现示例 ===");
    component_discovery_example().await?;

    println!("\n=== 手动注册示例 ===");
    manual_component_registration().await?;

    Ok(())
}
