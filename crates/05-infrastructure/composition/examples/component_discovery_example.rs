//! 组件发现机制使用示例
//!
//! 演示如何使用过程宏和反射机制进行组件注册和发现

use async_trait::async_trait;
use infrastructure_common::{Component, ComponentScope, Discoverable, DiscoveryMetadata, TypeInfo};
use infrastructure_composition::ComponentScannerBuilder;
use std::sync::Arc;

// 示例：使用手动注册的组件
#[derive(Debug)]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    cache: Arc<RedisCacheService>, // 使用具体类型而不是 trait object
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
    async fn get_user(&self, key: &str) -> Result<Option<User>, Box<dyn std::error::Error>>;
    async fn set_user(&self, key: &str, value: &User) -> Result<(), Box<dyn std::error::Error>>;
}

// 示例：用户实体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

// 示例：使用依赖注入的构造函数
#[inject]
impl UserService {
    pub fn new(repository: Arc<dyn UserRepository>, cache: Arc<dyn CacheService>) -> Self {
        Self { repository, cache }
    }

    pub async fn get_user(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>> {
        // 先尝试从缓存获取
        let cache_key = format!("user:{}", id);
        if let Ok(Some(user)) = self.cache.get::<User>(&cache_key).await {
            return Ok(Some(user));
        }

        // 从数据库获取
        if let Some(user) = self.repository.find_by_id(id).await? {
            // 缓存结果
            let _ = self.cache.set(&cache_key, &user).await;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
}

// 实现 Component trait
impl Component for UserService {
    fn name(&self) -> &'static str {
        "user_service"
    }
}

// 示例：MySQL Repository 实现
#[component(name = "mysql_user_repository", version = "1.0.0", scope = "singleton")]
#[derive(Debug, Discoverable)]
pub struct MySqlUserRepository {
    connection_pool: Arc<sqlx::MySqlPool>,
}

#[inject]
impl MySqlUserRepository {
    pub fn new(connection_pool: Arc<sqlx::MySqlPool>) -> Self {
        Self { connection_pool }
    }
}

impl Component for MySqlUserRepository {
    fn name(&self) -> &'static str {
        "mysql_user_repository"
    }
}

#[async_trait]
impl UserRepository for MySqlUserRepository {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let user = sqlx::query_as!(User, "SELECT id, name, email FROM users WHERE id = ?", id)
            .fetch_optional(&*self.connection_pool)
            .await?;

        Ok(user)
    }

    async fn save(&self, user: &User) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query!(
            "INSERT INTO users (id, name, email) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE name = VALUES(name), email = VALUES(email)",
            user.id,
            user.name,
            user.email
        )
        .execute(&*self.connection_pool)
        .await?;

        Ok(())
    }
}

// 示例：Redis 缓存服务实现
#[component(name = "redis_cache_service", version = "1.0.0", scope = "singleton")]
#[derive(Debug, Discoverable)]
pub struct RedisCacheService {
    client: Arc<redis::Client>,
}

#[inject]
impl RedisCacheService {
    pub fn new(client: Arc<redis::Client>) -> Self {
        Self { client }
    }
}

impl Component for RedisCacheService {
    fn name(&self) -> &'static str {
        "redis_cache_service"
    }
}

#[async_trait]
impl CacheService for RedisCacheService {
    async fn get<T>(&self, key: &str) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut conn = self.client.get_async_connection().await?;
        let value: Option<String> = redis::cmd("GET").arg(key).query_async(&mut conn).await?;

        if let Some(json_str) = value {
            let data: T = serde_json::from_str(&json_str)?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    async fn set<T>(&self, key: &str, value: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: serde::Serialize,
    {
        let json_str = serde_json::to_string(value)?;
        let mut conn = self.client.get_async_connection().await?;
        redis::cmd("SET")
            .arg(key)
            .arg(json_str)
            .arg("EX")
            .arg(3600) // 1小时过期
            .query_async(&mut conn)
            .await?;

        Ok(())
    }
}

// 示例：服务提供者
#[service_provider]
pub fn create_mysql_connection_pool() -> Arc<sqlx::MySqlPool> {
    // 实际实现中应该从配置读取连接信息
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://user:password@localhost/database".to_string());

    let pool = sqlx::MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to MySQL");

    Arc::new(pool)
}

#[service_provider]
pub fn create_redis_client() -> Arc<redis::Client> {
    // 实际实现中应该从配置读取连接信息
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    Arc::new(client)
}

/// 组件发现和注册示例
pub async fn component_discovery_example() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // 创建组件扫描器
    let mut scanner = ComponentScannerBuilder::new()
        // 添加发现策略
        .with_reflection_discovery()
        .with_annotation_discovery(vec!["crate".to_string()])
        .with_configuration_discovery(vec!["components.json".to_string()])
        // 添加过滤器
        .with_tag_filter(
            TagFilter::new()
                .include_tag("service")
                .include_tag("repository"),
        )
        .with_scope_filter(
            ScopeFilter::new()
                .allow_scope(ComponentScope::Singleton)
                .allow_scope(ComponentScope::Application),
        )
        // 添加拦截器
        .with_logging_interceptor()
        .with_performance_interceptor()
        .build();

    // 执行组件扫描
    println!("开始组件扫描...");
    let component_count = scanner.scan("crate").await?;
    println!("发现并注册了 {} 个组件", component_count);

    // 验证组件依赖关系
    let validation_issues = scanner.validate().await?;
    if validation_issues.is_empty() {
        println!("所有组件依赖关系验证通过");
    } else {
        println!("发现依赖关系问题:");
        for issue in validation_issues {
            println!("  - {}", issue);
        }
    }

    // 获取启动顺序
    let manager = scanner.get_manager();
    if let Ok(startup_order) = manager.get_startup_order() {
        println!("组件启动顺序:");
        for (index, type_info) in startup_order.iter().enumerate() {
            println!("  {}. {}", index + 1, type_info.name);
        }
    }

    // 查看依赖关系图
    let dependency_graph = manager.get_dependency_graph();
    println!("依赖关系图统计:");
    println!(
        "  - 组件总数: {}",
        dependency_graph.get_all_components().len()
    );

    // 按标签查找组件
    let service_components = dependency_graph.find_by_tag("service");
    println!("  - 服务组件: {}", service_components.len());

    let singleton_components = dependency_graph.find_by_scope(&ComponentScope::Singleton);
    println!("  - 单例组件: {}", singleton_components.len());

    // 发现特定类型的组件
    let user_services = scanner
        .discover(
            &[
                ("scope".to_string(), "singleton".to_string()),
                ("tag".to_string(), "service".to_string()),
            ]
            .into_iter()
            .collect(),
        )
        .await?;

    println!("用户服务组件:");
    for service in user_services {
        println!("  - {}", service);
    }

    Ok(())
}

/// 手动注册组件示例
pub async fn manual_component_registration() -> Result<(), Box<dyn std::error::Error>> {
    use infrastructure_common::GLOBAL_COMPONENT_REGISTRY;

    // 手动注册组件元数据
    let user_service_metadata = DiscoveryMetadata::new(
        TypeInfo::of::<UserService>(),
        "user_service".to_string(),
        vec![
            TypeInfo::of::<dyn UserRepository>(),
            TypeInfo::of::<dyn CacheService>(),
        ],
    )
    .provides::<dyn UserService>()
    .singleton()
    .with_tag("service")
    .with_tag("business_logic")
    .with_startup_order(10);

    GLOBAL_COMPONENT_REGISTRY
        .register_discovery_metadata(user_service_metadata)
        .await?;

    let repository_metadata = DiscoveryMetadata::new(
        TypeInfo::of::<MySqlUserRepository>(),
        "mysql_user_repository".to_string(),
        vec![TypeInfo::of::<sqlx::MySqlPool>()],
    )
    .provides::<dyn UserRepository>()
    .singleton()
    .with_tag("repository")
    .with_tag("database")
    .with_startup_order(5);

    GLOBAL_COMPONENT_REGISTRY
        .register_discovery_metadata(repository_metadata)
        .await?;

    println!("手动注册组件完成");

    // 查询已注册的组件
    let all_components = GLOBAL_COMPONENT_REGISTRY
        .get_all_discovery_metadata()
        .await?;
    println!("已注册组件数量: {}", all_components.len());

    for component in all_components {
        println!(
            "  - {} ({})",
            component.component_name, component.reflection.type_name
        );
        println!(
            "    依赖: {:?}",
            component
                .dependencies
                .iter()
                .map(|d| &d.name)
                .collect::<Vec<_>>()
        );
        println!("    标签: {:?}", component.tags);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_component_discovery() {
        let result = component_discovery_example().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_manual_registration() {
        let result = manual_component_registration().await;
        assert!(result.is_ok());
    }
}
