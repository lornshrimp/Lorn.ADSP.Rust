//! 依赖注入实现的集成测试

use di_abstractions::ComponentRegistry;
use di_impl::DiContainerImpl;
use infrastructure_common::{Component, Lifetime};
use std::sync::Arc;

/// 测试组件
#[derive(Debug)]
struct TestService {
    name: String,
}

impl Component for TestService {
    fn name(&self) -> &'static str {
        "TestService"
    }

    fn priority(&self) -> i32 {
        0
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

impl TestService {
    fn new(name: String) -> Self {
        Self { name }
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

#[tokio::test]
async fn test_component_registration_and_resolution() {
    let mut container = DiContainerImpl::new();

    // 测试实例注册
    let test_service = TestService::new("test".to_string());
    ComponentRegistry::register_instance(&mut container, test_service)
        .await
        .unwrap();

    // 测试组件是否已注册
    assert!(ComponentRegistry::is_registered::<TestService>(&container));

    // 测试组件解析
    let resolved_service = ComponentRegistry::resolve::<TestService>(&container)
        .await
        .unwrap();
    assert_eq!(resolved_service.get_name(), "test");
    assert_eq!(resolved_service.name(), "TestService");
}

#[tokio::test]
async fn test_factory_registration() {
    let mut container = DiContainerImpl::new();

    // 测试工厂注册
    ComponentRegistry::register_factory::<TestService, _>(
        &mut container,
        || Ok(TestService::new("factory_created".to_string())),
        Lifetime::Singleton,
    )
    .await
    .unwrap();

    // 测试组件是否已注册
    assert!(ComponentRegistry::is_registered::<TestService>(&container));

    // 测试组件解析
    let resolved_service = ComponentRegistry::resolve::<TestService>(&container)
        .await
        .unwrap();
    assert_eq!(resolved_service.get_name(), "factory_created");

    // 测试单例行为 - 第二次解析应该返回同一个实例
    let resolved_service2 = ComponentRegistry::resolve::<TestService>(&container)
        .await
        .unwrap();
    assert_eq!(resolved_service2.get_name(), "factory_created");

    // 验证是同一个实例（通过指针比较）
    assert!(Arc::ptr_eq(&resolved_service, &resolved_service2));
}

#[tokio::test]
async fn test_container_validation() {
    let mut container = DiContainerImpl::new();

    // 注册一个有效的组件
    let test_service = TestService::new("valid".to_string());
    ComponentRegistry::register_instance(&mut container, test_service)
        .await
        .unwrap();

    // 验证容器状态
    assert!(ComponentRegistry::validate_dependencies(&container)
        .await
        .is_ok());

    // 测试获取已注册组件列表
    let components = ComponentRegistry::get_registered_components(&container);
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].name, "integration_test::TestService");
}

#[tokio::test]
async fn test_container_clear() {
    let mut container = DiContainerImpl::new();

    // 注册一些组件
    let test_service = TestService::new("test".to_string());
    ComponentRegistry::register_instance(&mut container, test_service)
        .await
        .unwrap();

    assert!(ComponentRegistry::is_registered::<TestService>(&container));

    // 清理容器
    ComponentRegistry::clear(&mut container).await.unwrap();

    // 验证组件已被清理
    assert!(!ComponentRegistry::is_registered::<TestService>(&container));

    let components = ComponentRegistry::get_registered_components(&container);
    assert_eq!(components.len(), 0);
}
