# Lorn ADSP 示例应用

这是一个完整的示例应用，演示了如何使用 Lorn ADSP 统一配置化和依赖注入系统。

## 功能特性

- ✅ 统一配置管理（TOML、JSON、环境变量）
- ✅ 依赖注入容器
- ✅ 组件生命周期管理
- ✅ 健康检查系统
- ✅ 配置热重载
- ✅ 结构化日志记录
- ✅ 命令行接口

## 快速开始

### 1. 运行应用

```bash
# 使用默认配置运行
cargo run

# 指定配置文件
cargo run -- --config config/app.toml

# 启用热重载
cargo run -- --hot-reload

# 设置日志级别
cargo run -- --log-level debug
```

### 2. 查看帮助

```bash
cargo run -- --help
```

## 配置文件

### TOML 配置 (config/app.toml)

```toml
[app]
name = "Lorn ADSP 示例应用"
version = "0.1.0"
port = 8080
worker_threads = 4

[database]
host = "localhost"
port = 5432
database = "lorn_adsp"
username = "postgres"
password = "your_password_here"
max_connections = 20

[services.example_service]
enabled = true
timeout_seconds = 30
retry_count = 5
```

### JSON 配置 (config/app.json)

```json
{
  "app": {
    "name": "Lorn ADSP 示例应用 (JSON)",
    "version": "0.1.0",
    "port": 8081,
    "worker_threads": 6
  },
  "services": {
    "example_service": {
      "enabled": true,
      "timeout_seconds": 25,
      "retry_count": 3
    }
  }
}
```

### 环境变量配置

```bash
# 应用配置
export ADSP_APP_NAME="环境变量应用"
export ADSP_APP_PORT=8082

# 数据库配置
export ADSP_DATABASE_HOST=localhost
export ADSP_DATABASE_PORT=5432
export ADSP_DATABASE_DATABASE=adsp_env

# 服务配置
export ADSP_SERVICES_EXAMPLE_SERVICE_ENABLED=true
export ADSP_SERVICES_EXAMPLE_SERVICE_TIMEOUT_SECONDS=20
```

## 架构说明

### 基础设施层次

1. **配置管理**: 多源配置支持，类型安全，热重载
2. **依赖注入**: 自动组件扫描，生命周期管理
3. **健康检查**: 组件健康监控，整体状态报告
4. **错误处理**: 统一错误类型，详细错误信息

### 组件示例

```rust
// 实现 Component trait
impl Component for ExampleService {
    fn name(&self) -> &'static str {
        "ExampleService"
    }
    
    fn priority(&self) -> i32 {
        100
    }
}

// 实现 Configurable trait
impl Configurable for ExampleService {
    type Config = ExampleServiceConfig;
    
    fn configure(&mut self, config: Self::Config) -> Result<(), ConfigError> {
        self.config = Some(config);
        Ok(())
    }
    
    fn get_config_path() -> &'static str {
        "services.example_service"
    }
}
```

## 运行输出示例

```
2024-01-15T10:30:00.123Z  INFO example_app: 启动 Lorn ADSP 示例应用
2024-01-15T10:30:00.124Z  INFO example_app: 构建基础设施
2024-01-15T10:30:00.125Z  INFO example_app: 基础设施构建完成
2024-01-15T10:30:00.126Z  INFO example_app: 演示配置获取功能
2024-01-15T10:30:00.127Z  INFO example_app: 获取应用配置成功: AppConfig { name: "Lorn ADSP 示例应用", version: "0.1.0", port: 8080, worker_threads: 4 }
2024-01-15T10:30:00.128Z  INFO example_app: 演示组件解析功能
2024-01-15T10:30:00.129Z  INFO example_app: ExampleService 未注册，创建临时实例
2024-01-15T10:30:00.130Z  INFO example_app: ExampleService 正在执行工作
2024-01-15T10:30:00.231Z  INFO example_app: ExampleService 工作完成
2024-01-15T10:30:00.232Z  INFO example_app: 演示健康检查功能
2024-01-15T10:30:00.242Z  INFO example_app: 整体健康状态: Healthy
```

## 开发说明

### 添加新组件

1. 实现 `Component` trait
2. 可选：实现 `Configurable` trait 用于配置支持
3. 可选：实现 `HealthCheckable` trait 用于健康检查
4. 在 `AdSystemInfrastructure::builder()` 中注册组件

### 配置验证

配置系统支持类型安全的配置验证：

```rust
#[derive(Debug, Deserialize)]
struct MyConfig {
    #[serde(default = "default_port")]
    port: u16,
    
    #[serde(deserialize_with = "validate_positive")]
    threads: usize,
}

fn default_port() -> u16 { 8080 }
fn validate_positive<'de, D>(deserializer: D) -> Result<usize, D::Error> 
where D: serde::Deserializer<'de> {
    // 验证逻辑
}
```

### 热重载

启用热重载后，配置文件的更改会自动重新加载：

```rust
// 启用热重载
let infrastructure = AdSystemInfrastructure::builder()
    .add_config_toml("config/app.toml")?
    .enable_hot_reload(true)
    .build()
    .await?;
```

## 故障排除

### 常见问题

1. **配置文件找不到**: 检查文件路径是否正确
2. **组件注册失败**: 确保实现了必要的 trait
3. **配置解析错误**: 检查配置格式和字段类型
4. **健康检查超时**: 调整超时设置或优化检查逻辑

### 调试技巧

```bash
# 启用调试日志
RUST_LOG=debug cargo run

# 启用所有日志
RUST_LOG=trace cargo run

# 仅显示特定模块的日志
RUST_LOG=example_app=debug cargo run
```
