//! 组件基础接口定义
//! 
//! 提供所有基础设施组件必须实现的基础 trait

use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;

/// 组件基础 trait
/// 
/// 所有基础设施组件都必须实现此 trait
pub trait Component: Send + Sync + Debug + 'static {
    /// 组件名称
    fn name(&self) -> &'static str;
    
    /// 组件优先级，数值越高优先级越高
    fn priority(&self) -> i32 {
        0
    }
    
    /// 组件是否启用
    fn is_enabled(&self) -> bool {
        true
    }
    
    /// 组件类型ID
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// 可注入组件 trait
/// 
/// 支持依赖注入的组件必须实现此 trait
#[async_trait]
pub trait Injectable: Send + Sync + 'static {
    /// 依赖类型
    type Dependencies;
    
    /// 构建错误类型
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// 使用依赖注入构建组件实例
    async fn inject(deps: Self::Dependencies) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// 组件基础实现
#[derive(Debug)]
pub struct ComponentBase {
    pub name: &'static str,
    pub priority: i32,
    pub enabled: bool,
}

impl ComponentBase {
    /// 创建新的组件基础实例
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            priority: 0,
            enabled: true,
        }
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// 设置启用状态
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for ComponentBase {
    fn name(&self) -> &'static str {
        self.name
    }
    
    fn priority(&self) -> i32 {
        self.priority
    }
    
    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// 组件描述符
#[derive(Debug, Clone)]
pub struct ComponentDescriptor {
    /// 组件名称
    pub name: String,
    /// 组件类型ID
    pub type_id: TypeId,
    /// 组件生命周期
    pub lifetime: crate::lifecycle::Lifetime,
    /// 组件优先级
    pub priority: i32,
    /// 是否启用
    pub enabled: bool,
    /// 组件元数据
    pub metadata: HashMap<String, String>,
}

impl ComponentDescriptor {
    /// 创建新的组件描述符
    pub fn new<T: Component + 'static>(
        name: impl Into<String>,
        lifetime: crate::lifecycle::Lifetime,
    ) -> Self {
        Self {
            name: name.into(),
            type_id: TypeId::of::<T>(),
            lifetime,
            priority: 0,
            enabled: true,
            metadata: HashMap::new(),
        }
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// 设置启用状态
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// 组件工厂函数类型
pub type ComponentFactory = Box<
    dyn Fn(&dyn Any) -> Result<Box<dyn Any + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>
        + Send
        + Sync,
>;
