//! 组件发现机制简化示例
//!
//! 演示如何使用反射机制进行组件注册和发现

use async_trait::async_trait;
use infrastructure_common::{Component, DiscoveryMetadata, TypeInfo};
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

    pub async fn get_user(&self, key: &str) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let cache = self.cache.lock().unwrap();
        if let Some(json_str) = cache.get(key) {
            let user: User = serde_json::from_str(json_str)?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub async fn set_user(
        &self,
        key: &str,
        value: &User,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_str = serde_json::to_string(value)?;
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key.to_string(), json_str);
        Ok(())
    }
}

impl Component for MockCacheService {
    fn name(&self) -> &'static str {
        "mock_cache_service"
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
    use infrastructure_common::GLOBAL_COMPONENT_REGISTRY;

    // 手动注册组件元数据
    let user_service_metadata = DiscoveryMetadata::new(
        TypeInfo::of::<UserService>(),
        "user_service".to_string(),
        vec![
            TypeInfo::of::<MockUserRepository>(),
            TypeInfo::of::<MockCacheService>(),
        ],
    )
    .singleton()
    .with_tag("service")
    .with_tag("business_logic")
    .with_startup_order(10);

    GLOBAL_COMPONENT_REGISTRY
        .register_discovery_metadata(user_service_metadata)
        .await?;

    let repository_metadata = DiscoveryMetadata::new(
        TypeInfo::of::<MockUserRepository>(),
        "mock_user_repository".to_string(),
        vec![],
    )
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

// 添加 main 函数
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 组件发现示例 ===");
    component_discovery_example().await?;

    println!("\n=== 手动注册示例 ===");
    manual_component_registration().await?;

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
