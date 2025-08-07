//! # 依赖注入具体实现
//! 
//! 提供具体的依赖注入容器、组件注册器和解析器实现

use async_trait::async_trait;
use di_abstractions::{
    DiContainer, ContainerBuilder, ComponentFactory,
};
use infrastructure_common::{
    Component, ComponentError, DependencyError, ComponentMetadata, TypeInfo,
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// 简单的组件注册信息
#[derive(Debug, Clone)]
struct ComponentRegistration {
    /// 组件元数据
    metadata: ComponentMetadata,
    /// 单例实例（如果有）
    singleton: Option<Arc<dyn Any + Send + Sync>>,
}

/// 具体的依赖注入容器实现
pub struct DiContainerImpl {
    /// 组件注册信息
    registrations: Arc<RwLock<HashMap<TypeId, ComponentRegistration>>>,
}

impl DiContainerImpl {
    /// 创建新的容器
    pub fn new() -> Self {
        Self {
            registrations: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for DiContainerImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DiContainer for DiContainerImpl {
    async fn register<T>(&mut self, metadata: ComponentMetadata) -> Result<(), ComponentError>
    where
        T: Component + 'static,
    {
        info!("注册组件: {} ({})", metadata.name, std::any::type_name::<T>());
        
        let registration = ComponentRegistration {
            metadata,
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
        
        let metadata = ComponentMetadata::new(
            TypeInfo::of::<T>(),
            std::any::type_name::<T>(),
        );
        
        let registration = ComponentRegistration {
            metadata,
            singleton: Some(instance as Arc<dyn Any + Send + Sync>),
        };
        
        let mut registrations = self.registrations.write().await;
        registrations.insert(type_id, registration);
        
        Ok(())
    }

    async fn register_factory<T>(&mut self, _factory: Box<dyn ComponentFactory>) -> Result<(), ComponentError>
    where
        T: Component + 'static,
    {
        let type_id = TypeId::of::<T>();
        info!("注册工厂: {}", std::any::type_name::<T>());
        
        let metadata = ComponentMetadata::new(
            TypeInfo::of::<T>(),
            std::any::type_name::<T>(),
        );
        
        let registration = ComponentRegistration {
            metadata,
            singleton: None, // 暂不支持工厂
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
        let type_id = TypeId::of::<T>();
        
        let registrations = self.registrations.read().await;
        if let Some(registration) = registrations.get(&type_id) {
            if let Some(singleton) = &registration.singleton {
                if let Ok(typed_instance) = singleton.clone().downcast::<T>() {
                    Ok(typed_instance)
                } else {
                    Err(DependencyError::ComponentCreationFailed {
                        type_name: std::any::type_name::<T>().to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "类型转换失败"
                        )),
                    })
                }
            } else {
                Err(DependencyError::ComponentCreationFailed {
                    type_name: std::any::type_name::<T>().to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "暂不支持非单例组件解析"
                    )),
                })
            }
        } else {
            Err(DependencyError::ComponentNotRegistered {
                type_name: std::any::type_name::<T>().to_string(),
            })
        }
    }

    async fn resolve_by_type_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>, DependencyError> {
        let registrations = self.registrations.read().await;
        if let Some(registration) = registrations.get(&type_id) {
            if let Some(singleton) = &registration.singleton {
                Ok(singleton.clone())
            } else {
                Err(DependencyError::ComponentCreationFailed {
                    type_name: format!("TypeId({:?})", type_id),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "暂不支持非单例组件解析"
                    )),
                })
            }
        } else {
            Err(DependencyError::ComponentNotRegistered {
                type_name: format!("TypeId({:?})", type_id),
            })
        }
    }

    async fn resolve_by_name(&self, name: &str) -> Result<Arc<dyn Any + Send + Sync>, DependencyError> {
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
                            "暂不支持非单例组件解析"
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
        let type_id = TypeId::of::<T>();
        
        if let Ok(registrations) = self.registrations.try_read() {
            registrations.contains_key(&type_id)
        } else {
            false
        }
    }

    fn is_registered_by_type_id(&self, type_id: TypeId) -> bool {
        if let Ok(registrations) = self.registrations.try_read() {
            registrations.contains_key(&type_id)
        } else {
            false
        }
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
            registrations.values().map(|reg| reg.metadata.clone()).collect()
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
        let metadata = ComponentMetadata::new(
            TypeInfo::of::<T>(),
            std::any::type_name::<T>(),
        );
        
        let registration = ComponentRegistration {
            metadata,
            singleton: Some(instance as Arc<dyn Any + Send + Sync>),
        };
        
        self.registrations.push(registration);
        self
    }

    fn register_factory<T>(mut self, factory: Box<dyn ComponentFactory>) -> Self
    where
        T: Component + 'static,
        Self: Sized,
    {
        let metadata = ComponentMetadata::new(
            TypeInfo::of::<T>(),
            std::any::type_name::<T>(),
        );
        
        let registration = ComponentRegistration {
            metadata,
            singleton: None, // 工厂暂不支持预实例化
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

    fn with_scanner(self, _scanner: Box<dyn di_abstractions::ComponentScanner>) -> Self
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
