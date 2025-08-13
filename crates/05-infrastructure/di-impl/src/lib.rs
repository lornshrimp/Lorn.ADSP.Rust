//! # 依赖注入具体实现
//!
//! 提供具体的依赖注入容器、组件注册器和解析器实现

use async_trait::async_trait;
use di_abstractions::{
    ComponentFactory, ComponentRegistry, ComponentResolver, ComponentScanner, ContainerBuilder,
    DiContainer,
};
use infrastructure_common::{
    Component, ComponentDescriptor, ComponentError, ComponentMetadata, DependencyError, Lifetime,
    Scope, TypeInfo,
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 具体的依赖注入容器实现
pub struct DiContainerImpl {
    /// 组件注册信息
    registrations: Arc<RwLock<HashMap<TypeId, ComponentRegistration>>>,
    /// 单例实例缓存
    singletons: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

/// 简单的组件注册信息
#[derive(Clone)]
struct ComponentRegistration {
    /// 组件元数据
    metadata: ComponentMetadata,
    /// 组件工厂函数
    factory:
        Option<Arc<dyn Fn() -> Result<Arc<dyn Any + Send + Sync>, ComponentError> + Send + Sync>>,
    /// 生命周期
    lifetime: Lifetime,
    /// 单例实例（如果有）
    singleton: Option<Arc<dyn Any + Send + Sync>>,
}

impl std::fmt::Debug for ComponentRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentRegistration")
            .field("metadata", &self.metadata)
            .field("lifetime", &self.lifetime)
            .field("singleton", &self.singleton)
            .field("factory", &self.factory.as_ref().map(|_| "<function>"))
            .finish()
    }
}

impl DiContainerImpl {
    /// 创建新的容器
    pub fn new() -> Self {
        Self {
            registrations: Arc::new(RwLock::new(HashMap::new())),
            singletons: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for DiContainerImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ComponentRegistry for DiContainerImpl {
    async fn register_component<T>(&mut self, lifetime: Lifetime) -> Result<(), DependencyError>
    where
        T: Component + 'static,
    {
        let type_id = TypeId::of::<T>();
        info!("注册组件: {} ({:?})", std::any::type_name::<T>(), lifetime);

        let metadata = ComponentMetadata::new(TypeInfo::of::<T>(), std::any::type_name::<T>());

        let registration = ComponentRegistration {
            metadata,
            factory: None, // 需要工厂函数才能创建实例
            lifetime,
            singleton: None,
        };

        let mut registrations = self.registrations.write().await;
        registrations.insert(type_id, registration);

        Ok(())
    }

    async fn register_instance<T>(&mut self, instance: T) -> Result<(), DependencyError>
    where
        T: Component + 'static,
    {
        let type_id = TypeId::of::<T>();
        info!("注册单例实例: {}", std::any::type_name::<T>());

        let metadata = ComponentMetadata::new(TypeInfo::of::<T>(), std::any::type_name::<T>());

        let instance_arc = Arc::new(instance) as Arc<dyn Any + Send + Sync>;

        let registration = ComponentRegistration {
            metadata,
            factory: None,
            lifetime: Lifetime::Singleton,
            singleton: Some(instance_arc.clone()),
        };

        let mut registrations = self.registrations.write().await;
        registrations.insert(type_id, registration);

        let mut singletons = self.singletons.write().await;
        singletons.insert(type_id, instance_arc);

        Ok(())
    }

    async fn register_factory<T, F>(
        &mut self,
        factory: F,
        lifetime: Lifetime,
    ) -> Result<(), DependencyError>
    where
        T: Component + 'static,
        F: Fn() -> Result<T, DependencyError> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        info!("注册工厂: {} ({:?})", std::any::type_name::<T>(), lifetime);

        let metadata = ComponentMetadata::new(TypeInfo::of::<T>(), std::any::type_name::<T>());

        // 包装工厂函数以返回 Arc<dyn Any>
        let wrapped_factory = Arc::new(
            move || -> Result<Arc<dyn Any + Send + Sync>, ComponentError> {
                let instance = factory().map_err(|e| ComponentError::FactoryCreationError {
                    type_name: std::any::type_name::<T>().to_string(),
                    message: e.to_string(),
                })?;
                Ok(Arc::new(instance) as Arc<dyn Any + Send + Sync>)
            },
        );

        let registration = ComponentRegistration {
            metadata,
            factory: Some(wrapped_factory),
            lifetime,
            singleton: None,
        };

        let mut registrations = self.registrations.write().await;
        registrations.insert(type_id, registration);

        Ok(())
    }

    async fn resolve<T>(&self) -> Result<Arc<T>, DependencyError>
    where
        T: Component + 'static,
    {
        let type_id = TypeId::of::<T>();

        // 首先检查单例缓存
        {
            let singletons = self.singletons.read().await;
            if let Some(instance) = singletons.get(&type_id) {
                return instance.clone().downcast::<T>().map_err(|_| {
                    DependencyError::ComponentCreationFailed {
                        type_name: std::any::type_name::<T>().to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "类型转换失败",
                        )),
                    }
                });
            }
        }

        // 然后检查注册信息
        let registrations = self.registrations.read().await;
        if let Some(registration) = registrations.get(&type_id) {
            if let Some(factory) = &registration.factory {
                let instance = factory().map_err(|e| DependencyError::ComponentCreationFailed {
                    type_name: std::any::type_name::<T>().to_string(),
                    source: Box::new(e),
                })?;

                let typed_instance = instance.downcast::<T>().map_err(|_| {
                    DependencyError::ComponentCreationFailed {
                        type_name: std::any::type_name::<T>().to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "类型转换失败",
                        )),
                    }
                })?;

                // 如果是单例，缓存实例
                if matches!(registration.lifetime, Lifetime::Singleton) {
                    let mut singletons = self.singletons.write().await;
                    singletons.insert(
                        type_id,
                        typed_instance.clone() as Arc<dyn Any + Send + Sync>,
                    );
                }

                Ok(typed_instance)
            } else if let Some(singleton) = &registration.singleton {
                singleton.clone().downcast::<T>().map_err(|_| {
                    DependencyError::ComponentCreationFailed {
                        type_name: std::any::type_name::<T>().to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "类型转换失败",
                        )),
                    }
                })
            } else {
                Err(DependencyError::ComponentCreationFailed {
                    type_name: std::any::type_name::<T>().to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "没有可用的工厂或实例",
                    )),
                })
            }
        } else {
            Err(DependencyError::ComponentNotRegistered {
                type_name: std::any::type_name::<T>().to_string(),
            })
        }
    }

    async fn resolve_scoped<T>(&self, _scope: &Scope) -> Result<Arc<T>, DependencyError>
    where
        T: Component + 'static,
    {
        // 简单实现：暂时忽略作用域，直接调用 resolve
        warn!("作用域解析暂未完全实现，使用默认解析");
        ComponentRegistry::resolve(self).await
    }

    async fn resolve_all<T>(&self) -> Result<Vec<Arc<T>>, DependencyError>
    where
        T: Component + 'static,
    {
        // 简单实现：返回单个实例的向量
        match ComponentRegistry::resolve::<T>(self).await {
            Ok(instance) => Ok(vec![instance]),
            Err(_) => Ok(vec![]),
        }
    }

    fn is_registered<T>(&self) -> bool
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        ComponentRegistry::is_registered_by_type_id(self, type_id)
    }

    fn is_registered_by_type_id(&self, type_id: TypeId) -> bool {
        if let Ok(registrations) = self.registrations.try_read() {
            registrations.contains_key(&type_id)
        } else {
            false
        }
    }

    fn get_registered_components(&self) -> Vec<ComponentDescriptor> {
        if let Ok(registrations) = self.registrations.try_read() {
            registrations
                .values()
                .map(|reg| {
                    // 创建一个简单的描述符，不使用泛型方法
                    ComponentDescriptor {
                        name: reg.metadata.name.clone(),
                        type_id: reg.metadata.type_info.id,
                        lifetime: reg.lifetime,
                        priority: 0,
                        enabled: true,
                        metadata: std::collections::HashMap::new(),
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    async fn register_all_components(&mut self) -> Result<(), DependencyError> {
        info!("注册所有组件");
        // 简单实现：什么都不做
        Ok(())
    }

    async fn validate_dependencies(&self) -> Result<(), DependencyError> {
        info!("验证依赖关系");

        let registrations = self.registrations.read().await;
        for (type_id, registration) in registrations.iter() {
            debug!("验证组件: {} ({:?})", registration.metadata.name, type_id);

            // 简单验证：检查是否有工厂或实例
            if registration.factory.is_none() && registration.singleton.is_none() {
                error!("组件 {} 没有可用的工厂或实例", registration.metadata.name);
                return Err(DependencyError::ComponentCreationFailed {
                    type_name: registration.metadata.name.clone(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "没有可用的工厂或实例",
                    )),
                });
            }
        }

        Ok(())
    }

    async fn clear(&mut self) -> Result<(), DependencyError> {
        info!("清理所有注册的组件");

        let mut registrations = self.registrations.write().await;
        registrations.clear();

        let mut singletons = self.singletons.write().await;
        singletons.clear();

        Ok(())
    }
}

#[async_trait]
impl DiContainer for DiContainerImpl {
    async fn register<T>(&mut self, metadata: ComponentMetadata) -> Result<(), ComponentError>
    where
        T: Component + 'static,
    {
        info!(
            "注册组件: {} ({})",
            metadata.name,
            std::any::type_name::<T>()
        );

        let registration = ComponentRegistration {
            metadata,
            factory: None,
            lifetime: Lifetime::Transient,
            singleton: None,
        };

        let type_id = TypeId::of::<T>();
        let mut registrations = self.registrations.write().await;
        registrations.insert(type_id, registration);

        Ok(())
    }

    async fn register_singleton<T>(&mut self, instance: Arc<T>) -> Result<(), ComponentError>
    where
        T: Component + 'static,
    {
        let type_id = TypeId::of::<T>();
        info!("注册单例组件: {}", std::any::type_name::<T>());

        let metadata = ComponentMetadata::new(TypeInfo::of::<T>(), std::any::type_name::<T>());

        let registration = ComponentRegistration {
            metadata,
            factory: None,
            lifetime: Lifetime::Singleton,
            singleton: Some(instance.clone() as Arc<dyn Any + Send + Sync>),
        };

        let mut registrations = self.registrations.write().await;
        registrations.insert(type_id, registration);

        let mut singletons = self.singletons.write().await;
        singletons.insert(type_id, instance as Arc<dyn Any + Send + Sync>);

        Ok(())
    }

    async fn register_factory<T>(
        &mut self,
        _factory: Box<dyn ComponentFactory>,
    ) -> Result<(), ComponentError>
    where
        T: Component + 'static,
    {
        let type_id = TypeId::of::<T>();
        info!("注册工厂: {}", std::any::type_name::<T>());

        let metadata = ComponentMetadata::new(TypeInfo::of::<T>(), std::any::type_name::<T>());

        let registration = ComponentRegistration {
            metadata,
            factory: None, // 暂不支持工厂
            lifetime: Lifetime::Transient,
            singleton: None,
        };

        let mut registrations = self.registrations.write().await;
        registrations.insert(type_id, registration);

        warn!("工厂注册暂不完全支持，仅记录元数据");
        Ok(())
    }

    async fn resolve<T>(&self) -> Result<Arc<T>, DependencyError>
    where
        T: Component + 'static,
    {
        ComponentRegistry::resolve(self).await
    }

    async fn resolve_by_type_id(
        &self,
        type_id: TypeId,
    ) -> Result<Arc<dyn Any + Send + Sync>, DependencyError> {
        let registrations = self.registrations.read().await;
        if let Some(registration) = registrations.get(&type_id) {
            if let Some(singleton) = &registration.singleton {
                Ok(singleton.clone())
            } else {
                Err(DependencyError::ComponentCreationFailed {
                    type_name: format!("TypeId({:?})", type_id),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "暂不支持非单例组件解析",
                    )),
                })
            }
        } else {
            Err(DependencyError::ComponentNotRegistered {
                type_name: format!("TypeId({:?})", type_id),
            })
        }
    }

    async fn resolve_by_name(
        &self,
        name: &str,
    ) -> Result<Arc<dyn Any + Send + Sync>, DependencyError> {
        let registrations = self.registrations.read().await;
        for (_, registration) in registrations.iter() {
            if registration.metadata.name == name {
                if let Some(singleton) = &registration.singleton {
                    return Ok(singleton.clone());
                } else {
                    return Err(DependencyError::ComponentCreationFailed {
                        type_name: name.to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::Unsupported,
                            "暂不支持非单例组件解析",
                        )),
                    });
                }
            }
        }

        Err(DependencyError::ComponentNotRegistered {
            type_name: name.to_string(),
        })
    }

    fn is_registered<T>(&self) -> bool
    where
        T: Component + 'static,
    {
        ComponentRegistry::is_registered::<T>(self)
    }

    fn is_registered_by_type_id(&self, type_id: TypeId) -> bool {
        ComponentRegistry::is_registered_by_type_id(self, type_id)
    }

    fn is_registered_by_name(&self, name: &str) -> bool {
        if let Ok(registrations) = self.registrations.try_read() {
            registrations.values().any(|reg| reg.metadata.name == name)
        } else {
            false
        }
    }

    fn get_registered_components(&self) -> Vec<ComponentMetadata> {
        if let Ok(registrations) = self.registrations.try_read() {
            registrations
                .values()
                .map(|reg| reg.metadata.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    async fn scan_and_register(&mut self, target: &str) -> Result<usize, ComponentError> {
        info!("扫描目标: {}", target);
        // 简单实现，实际应该使用组件扫描器
        warn!("组件扫描功能需要进一步实现");
        Ok(0)
    }

    async fn validate(&self) -> Result<(), Vec<ComponentError>> {
        info!("验证容器状态");
        // 简单实现，检查是否有循环依赖等
        Ok(())
    }
}

/// 容器构建器实现
pub struct DiContainerBuilder {
    registrations: Vec<ComponentRegistration>,
    scan_targets: Vec<String>,
}

impl DiContainerBuilder {
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
            scan_targets: Vec::new(),
        }
    }
}

impl Default for DiContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerBuilder for DiContainerBuilder {
    type Container = DiContainerImpl;

    fn build(self) -> Result<Self::Container, ComponentError> {
        let container = DiContainerImpl::new();

        // 记录注册组件数量
        let registration_count = self.registrations.len();

        // 注册所有组件到容器中
        for registration in self.registrations {
            // 这里应该实际注册组件实例
            // 暂时只记录元数据
            info!("注册组件: {}", registration.metadata.name);
        }

        info!("构建容器完成，注册了 {} 个组件", registration_count);
        Ok(container)
    }

    fn register_component<T>(mut self, metadata: ComponentMetadata) -> Self
    where
        T: Component + 'static,
        Self: Sized,
    {
        let registration = ComponentRegistration {
            metadata,
            factory: None,
            lifetime: Lifetime::Transient,
            singleton: None,
        };

        self.registrations.push(registration);
        self
    }

    fn register_singleton<T>(mut self, instance: Arc<T>) -> Self
    where
        T: Component + 'static,
        Self: Sized,
    {
        let metadata = ComponentMetadata::new(TypeInfo::of::<T>(), std::any::type_name::<T>());

        let registration = ComponentRegistration {
            metadata,
            factory: None,
            lifetime: Lifetime::Singleton,
            singleton: Some(instance as Arc<dyn Any + Send + Sync>),
        };

        self.registrations.push(registration);
        self
    }

    fn register_factory<T>(mut self, _factory: Box<dyn ComponentFactory>) -> Self
    where
        T: Component + 'static,
        Self: Sized,
    {
        let metadata = ComponentMetadata::new(TypeInfo::of::<T>(), std::any::type_name::<T>());

        let registration = ComponentRegistration {
            metadata,
            factory: None, // 工厂暂不支持预实例化
            lifetime: Lifetime::Transient,
            singleton: None,
        };

        self.registrations.push(registration);
        info!("注册工厂: {}", std::any::type_name::<T>());
        self
    }

    fn add_scan_target(mut self, target: String) -> Self
    where
        Self: Sized,
    {
        self.scan_targets.push(target);
        self
    }

    fn with_scanner(self, _scanner: Box<dyn ComponentScanner>) -> Self
    where
        Self: Sized,
    {
        // TODO: 实现扫描器配置
        self
    }

    fn with_discovery(self, _discovery: Box<dyn di_abstractions::ComponentDiscovery>) -> Self
    where
        Self: Sized,
    {
        // TODO: 实现发现器配置
        self
    }
}
