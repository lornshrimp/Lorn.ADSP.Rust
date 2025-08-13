//! 组件发现机制使用示例（高级版，已迁移到工作区 examples）
//!
//! 演示如何使用过程宏和反射机制进行组件注册和发现

use async_trait::async_trait;
use component_macros::component;
use infrastructure_common::Component;
use std::sync::Arc;

// 示例：使用依赖注入的组件
#[derive(Debug)]
pub struct UserService {
    repository: Arc<MySqlUserRepository>,
    cache: Arc<RedisCacheService>,
}

// 示例：Repository trait
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>>;
    async fn save(&self, user: &User) -> Result<(), Box<dyn std::error::Error>>;
}

// 示例：缓存服务 trait
#[async_trait]
pub trait CacheService: Send + Sync {
    async fn get_user(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>>;
    async fn set_user(&self, user: &User) -> Result<(), Box<dyn std::error::Error>>;
}

// 示例：用户实体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

// 示例：使用依赖注入的构造函数
impl UserService {
    pub fn new(repository: Arc<MySqlUserRepository>, cache: Arc<RedisCacheService>) -> Self {
        Self { repository, cache }
    }
}

// 实现 Component trait
impl Component for UserService {
    fn name(&self) -> &'static str {
        "user_service"
    }
}

// 示例：MySQL Repository 实现
#[component(name = "mysql_user_repository")]
#[derive(Debug)]
pub struct MySqlUserRepository;
impl MySqlUserRepository {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl UserRepository for MySqlUserRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>> {
        Ok(Some(User {
            id,
            name: "demo".into(),
            email: "demo@example.com".into(),
        }))
    }
    async fn save(&self, _user: &User) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// 示例：Redis 缓存服务实现
#[component(name = "redis_cache_service")]
#[derive(Debug)]
pub struct RedisCacheService;
impl RedisCacheService {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl CacheService for RedisCacheService {
    async fn get_user(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>> {
        Ok(Some(User {
            id,
            name: "cached".into(),
            email: "cached@example.com".into(),
        }))
    }
    async fn set_user(&self, _user: &User) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// 简化：不演示外部资源 Provider，聚焦组件自动注册机制

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // 演示：构建并使用简单组件
    let repo = Arc::new(MySqlUserRepository::new());
    let cache = Arc::new(RedisCacheService::new());
    let service = UserService::new(repo.clone(), cache.clone());
    let user = service.repository.find_by_id(42).await?;
    println!("示例组件运行: {:?}", user);

    Ok(())
}
