# Component Macros

这个 crate 提供了用于自动组件注册和配置绑定的过程宏，是 Lorn.ADSP 核心基础架构的重要组成部分。

## 功能特性

- **自动组件注册**: 通过 `#[component]` 宏自动实现 `Component` trait 并注册到全局组件注册表
- **自动配置绑定**: 通过 `#[configurable]` 宏自动实现 `Configurable` trait 并提供配置绑定功能
- **生命周期管理**: 通过 `#[lifecycle]` 宏自动实现 `Lifecycle` trait 并管理组件生命周期
- **编译时安全**: 所有宏都在编译时进行验证，确保类型安全
- **约定优于配置**: 基于命名约定自动推断配置路径和生命周期

## 核心宏

### `#[component]` - 组件注册宏

自动为结构体实现 `Component` trait，并在程序启动时自动注册到全局组件注册表。

#### 参数

- `singleton` - 单例生命周期（默认）
- `scoped` - 作用域生命周期
- `transient` - 瞬态生命周期
- `priority = N` - 组件优先级（默认为 0）
- `name = "custom_name"` - 自定义组件名称
- `enabled` / `disabled` - 组件启用状态

#### 示例

```rust
use component_macros::component;
use infrastructure_common::Component;

// 基本用法
#[component]
pub struct MyService {
    // 字段
}

// 指定生命周期和优先级
#[component(singleton, priority = 100)]
pub struct DatabaseService {
    // 字段
}

// 自定义名称
#[component(scoped, name = "CustomName")]
pub struct CustomService {
    // 字段
}

// 瞬态组件
#[component(transient)]
pub struct TemporaryWorker {
    // 字段
}
```

### `#[configurable]` - 配置绑定宏

自动为结构体实现 `Configurable` trait，提供类型安全的配置绑定功能。

#### 参数

- `path = "config.path"` - 配置路径（必需）
- `optional` - 配置是否可选（默认为必需）
- `default` - 使用默认配置
- `field = "field_name"` - 指定配置字段名称

#### 示例

```rust
use component_macros::configurable;
use infrastructure_common::Configurable;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MyServiceConfig {
    pub enabled: bool,
    pub timeout: u64,
}

impl Default for MyServiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: 30,
        }
    }
}

// 基本用法
#[configurable(path = "services.my_service")]
pub struct MyService {
    config: Option<MyServiceConfig>,
}

// 可选配置
#[configurable(path = "services.optional_service", optional)]
pub struct OptionalService {
    config: Option<MyServiceConfig>,
}

// 使用默认配置
#[configurable(path = "services.default_service", default)]
pub struct DefaultService {
    config: Option<MyServiceConfig>,
}
```

### `#[lifecycle]` - 生命周期管理宏

自动为结构体实现 `Lifecycle` trait，提供组件生命周期管理功能。

#### 参数

- `on_start = "method_name"` - 启动时调用的方法
- `on_stop = "method_name"` - 停止时调用的方法
- `initialize = "method_name"` - 初始化方法（等同于 on_start）
- `cleanup = "method_name"` - 清理方法（等同于 on_stop）
- `depends_on = ["Service1", "Service2"]` - 依赖的组件列表
- `async` - 是否为异步生命周期

#### 示例

```rust
use component_macros::lifecycle;
use infrastructure_common::{Lifecycle, DependencyAware};

// 基本生命周期管理
#[lifecycle(
    on_start = "initialize",
    on_stop = "cleanup"
)]
pub struct MyService {
    initialized: bool,
}

impl MyService {
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.initialized = true;
        println!("Service initialized");
        Ok(())
    }

    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.initialized = false;
        println!("Service cleaned up");
        Ok(())
    }
}

// 带依赖的生命周期管理
#[lifecycle(
    on_start = "start_with_deps",
    on_stop = "stop_with_deps",
    depends_on = ["DatabaseService", "CacheService"]
)]
pub struct ComplexService {
    // 字段
}

// 异步生命周期管理
#[lifecycle(
    async,
    on_start = "async_initialize",
    on_stop = "async_cleanup"
)]
pub struct AsyncService {
    // 字段
}

impl AsyncService {
    pub async fn async_initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 异步初始化逻辑
        Ok(())
    }

    pub async fn async_cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 异步清理逻辑
        Ok(())
    }
}
```

## 组合使用

多个宏可以组合使用，提供完整的组件功能：

```rust
use component_macros::{component, configurable, lifecycle};
use infrastructure_common::{Component, Configurable, Lifecycle, DependencyAware};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub timeout: u64,
    pub max_connections: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: 30,
            max_connections: 100,
        }
    }
}

#[component(singleton, priority = 100)]
#[configurable(path = "services.full_service", default)]
#[lifecycle(
    on_start = "initialize",
    on_stop = "cleanup",
    depends_on = ["DatabaseService", "CacheService"]
)]
pub struct FullService {
    config: Option<ServiceConfig>,
    initialized: bool,
}

impl FullService {
    pub fn new() -> Self {
        Self {
            config: None,
            initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.initialized = true;
        println!("FullService initialized with config: {:?}", self.config);
        Ok(())
    }

    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.initialized = false;
        println!("FullService cleaned up");
        Ok(())
    }
}
```

## 派生宏

除了属性宏，还提供了派生宏用于更简单的场景：

```rust
use component_macros::{Component, Configurable};

// 派生 Component trait
#[derive(Debug, Component)]
#[component(priority = 50)]
pub struct SimpleService {
    name: String,
}

// 派生 Configurable trait
#[derive(Debug, Configurable)]
#[configurable(path = "services.simple")]
pub struct ConfigurableService {
    config: SimpleServiceConfig,
}
```

## 最佳实践

### 1. 命名约定

- 服务类组件使用 `*Service` 后缀，默认为单例生命周期
- 管理器组件使用 `*Manager` 后缀，默认为单例生命周期
- 提供者组件使用 `*Provider` 后缀，默认为作用域生命周期
- 策略组件使用 `*Strategy` 后缀，默认为瞬态生命周期

### 2. 配置管理

- 配置结构体使用 `*Config` 后缀
- 为配置结构体实现 `Default` trait
- 使用有意义的配置路径，如 `services.service_name`
- 对于可选配置使用 `optional` 参数

### 3. 生命周期管理

- 为需要初始化的组件实现启动和停止方法
- 明确声明组件依赖关系
- 对于异步操作使用 `async` 参数
- 确保清理方法能够正确释放资源

### 4. 错误处理

- 生命周期方法应该返回 `Result` 类型
- 使用具体的错误类型而不是泛型错误
- 在错误情况下提供有意义的错误信息

## 编译时检查

宏会在编译时进行以下检查：

1. **配置路径验证**: 确保配置路径格式正确
2. **方法存在性检查**: 验证生命周期方法是否存在
3. **类型兼容性检查**: 确保配置类型与字段类型兼容
4. **依赖关系验证**: 检查依赖组件是否存在

## 运行时行为

### 组件注册

使用 `#[component]` 宏的组件会在程序启动时自动注册到全局组件注册表。注册过程使用 `ctor` crate 在程序初始化阶段执行。

### 配置绑定

配置绑定在组件创建时进行，支持：
- 类型安全的配置绑定
- 配置验证
- 热重载支持（如果配置管理器支持）

### 生命周期管理

生命周期管理遵循以下顺序：
1. 依赖解析
2. 组件创建
3. 配置应用
4. 生命周期启动
5. 正常运行
6. 生命周期停止
7. 资源清理

## 性能考虑

- 宏在编译时展开，运行时无额外开销
- 组件注册使用高效的数据结构
- 配置绑定支持缓存机制
- 生命周期管理支持并行启动（在依赖允许的情况下）

## 故障排除

### 常见错误

1. **配置路径为空**: 确保为 `#[configurable]` 宏提供 `path` 参数
2. **方法不存在**: 确保生命周期方法在结构体中定义
3. **类型不匹配**: 确保配置类型实现了必要的 trait
4. **循环依赖**: 检查组件依赖关系，避免循环依赖

### 调试技巧

1. 使用 `cargo expand` 查看宏展开后的代码
2. 启用编译器的详细错误信息
3. 使用单元测试验证宏生成的代码
4. 检查全局组件注册表的状态

## 版本兼容性

- 支持 Rust 1.75+
- 兼容 `infrastructure-common` 0.1.0+
- 支持 `serde` 1.0+
- 支持 `tokio` 1.0+（用于异步生命周期）

## 许可证

MIT License - 详见 LICENSE 文件。