//! # 示例应用程序
//! 
//! 演示如何使用 Lorn ADSP 统一配置化和依赖注入系统

use infrastructure_composition::AdSystemInfrastructure;
use infrastructure_common::{Component, Configurable, HealthCheckable, HealthStatus};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use clap::Parser;
use tracing::{info, error};

/// 命令行参数
#[derive(Parser, Debug)]
#[command(name = "example-app")]
#[command(about = "Lorn ADSP 示例应用")]
struct Args {
    /// 配置文件路径
    #[arg(short, long, default_value = "config/app.toml")]
    config: String,
    
    /// 是否启用热重载
    #[arg(long)]
    hot_reload: bool,
    
    /// 日志级别
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(parse_log_level(&args.log_level))
        .init();
    
    info!("启动 Lorn ADSP 示例应用");
    
    // 构建基础设施
    let infrastructure = build_infrastructure(&args).await?;
    
    // 启动基础设施
    infrastructure.start().await?;
    
    // 演示配置获取
    demonstrate_configuration(&infrastructure).await?;
    
    // 演示组件解析
    demonstrate_component_resolution(&infrastructure).await?;
    
    // 演示健康检查
    demonstrate_health_check(&infrastructure).await?;
    
    // 等待退出信号
    tokio::signal::ctrl_c().await?;
    
    info!("收到退出信号，正在关闭应用");
    
    // 停止基础设施
    infrastructure.stop().await?;
    
    info!("应用已关闭");
    Ok(())
}

/// 构建基础设施
async fn build_infrastructure(args: &Args) -> Result<AdSystemInfrastructure, Box<dyn std::error::Error>> {
    info!("构建基础设施");
    
    let mut builder = AdSystemInfrastructure::builder();
    
    // 添加配置文件（如果存在）
    if std::path::Path::new(&args.config).exists() {
        if args.config.ends_with(".toml") {
            builder = builder.add_config_toml(&args.config)?;
        } else if args.config.ends_with(".json") {
            builder = builder.add_config_json(&args.config)?;
        }
    } else {
        info!("配置文件不存在，将使用默认配置和环境变量");
    }
    
    // 添加环境变量配置源
    builder = builder.add_config_env_vars("ADSP")?;
    
    // 配置热重载
    if args.hot_reload {
        builder = builder.enable_hot_reload(true);
    }
    
    // 启用健康检查
    builder = builder.enable_health_checks(true);
    
    // 添加示例组件扫描
    builder = builder.scan_crate("example-app")?;
    
    // 构建
    let infrastructure = builder.build().await?;
    
    info!("基础设施构建完成");
    Ok(infrastructure)
}

/// 演示配置获取
async fn demonstrate_configuration(infrastructure: &AdSystemInfrastructure) -> Result<(), Box<dyn std::error::Error>> {
    info!("演示配置获取功能");
    
    // 尝试获取应用配置
    match infrastructure.get_config::<AppConfig>("app").await {
        Ok(config) => {
            info!("获取应用配置成功: {:?}", config);
        }
        Err(e) => {
            info!("获取应用配置失败，使用默认配置: {}", e);
            let default_config = AppConfig::default();
            info!("默认应用配置: {:?}", default_config);
        }
    }
    
    // 尝试获取数据库配置
    match infrastructure.get_config::<DatabaseConfig>("database").await {
        Ok(config) => {
            info!("获取数据库配置成功: {:?}", config);
        }
        Err(e) => {
            info!("获取数据库配置失败，使用默认配置: {}", e);
            let default_config = DatabaseConfig::default();
            info!("默认数据库配置: {:?}", default_config);
        }
    }
    
    Ok(())
}

/// 演示组件解析
async fn demonstrate_component_resolution(infrastructure: &AdSystemInfrastructure) -> Result<(), Box<dyn std::error::Error>> {
    info!("演示组件解析功能");
    
    // 检查是否注册了示例服务
    if infrastructure.is_component_registered::<ExampleService>() {
        info!("ExampleService 已注册");
        
        match infrastructure.resolve::<ExampleService>().await {
            Ok(service) => {
                info!("解析 ExampleService 成功: {}", service.name());
                service.do_work().await?;
            }
            Err(e) => {
                error!("解析 ExampleService 失败: {}", e);
            }
        }
    } else {
        info!("ExampleService 未注册，创建临时实例");
        let service = ExampleService::new();
        service.do_work().await?;
    }
    
    Ok(())
}

/// 演示健康检查
async fn demonstrate_health_check(infrastructure: &AdSystemInfrastructure) -> Result<(), Box<dyn std::error::Error>> {
    info!("演示健康检查功能");
    
    // 执行健康检查
    let health_results = infrastructure.check_health().await;
    
    for (component_name, status) in health_results {
        match status {
            HealthStatus::Healthy => {
                info!("组件 {} 健康", component_name);
            }
            HealthStatus::Degraded { message, .. } => {
                info!("组件 {} 降级: {}", component_name, message);
            }
            HealthStatus::Unhealthy { error, .. } => {
                error!("组件 {} 不健康: {}", component_name, error);
            }
        }
    }
    
    // 获取整体健康状态
    let overall_health = infrastructure.get_overall_health().await;
    info!("整体健康状态: {:?}", overall_health);
    
    Ok(())
}

/// 解析日志级别
fn parse_log_level(level: &str) -> tracing::Level {
    match level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    }
}

// 示例配置结构

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 应用名称
    pub name: String,
    /// 应用版本
    pub version: String,
    /// 监听端口
    pub port: u16,
    /// 工作线程数
    pub worker_threads: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "example-app".to_string(),
            version: "0.1.0".to_string(),
            port: 8080,
            worker_threads: 4,
        }
    }
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库主机
    pub host: String,
    /// 数据库端口
    pub port: u16,
    /// 数据库名称
    pub database: String,
    /// 用户名
    pub username: String,
    /// 密码（在实际应用中应该加密存储）
    pub password: String,
    /// 最大连接数
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "adsp".to_string(),
            username: "postgres".to_string(),
            password: "password".to_string(),
            max_connections: 10,
        }
    }
}

// 示例组件

/// 示例服务
#[derive(Debug)]
pub struct ExampleService {
    name: String,
    config: Option<ExampleServiceConfig>,
}

impl ExampleService {
    /// 创建新的示例服务
    pub fn new() -> Self {
        Self {
            name: "ExampleService".to_string(),
            config: None,
        }
    }
    
    /// 执行工作
    pub async fn do_work(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("ExampleService 正在执行工作");
        
        if let Some(ref config) = self.config {
            info!("使用配置: {:?}", config);
        } else {
            info!("使用默认配置");
        }
        
        // 模拟一些工作
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        info!("ExampleService 工作完成");
        Ok(())
    }
}

impl Component for ExampleService {
    fn name(&self) -> &'static str {
        "ExampleService"
    }
    
    fn priority(&self) -> i32 {
        100
    }
}

/// 示例服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleServiceConfig {
    /// 是否启用
    pub enabled: bool,
    /// 超时时间（秒）
    pub timeout_seconds: u64,
    /// 重试次数
    pub retry_count: u32,
}

impl Default for ExampleServiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_seconds: 30,
            retry_count: 3,
        }
    }
}

impl Configurable for ExampleService {
    type Config = ExampleServiceConfig;
    
    fn configure(&mut self, config: Self::Config) -> Result<(), infrastructure_common::ConfigError> {
        info!("配置 ExampleService: {:?}", config);
        self.config = Some(config);
        Ok(())
    }
    
    fn get_config_path() -> &'static str {
        "services.example_service"
    }
}

/// 示例健康检查器
#[derive(Debug)]
pub struct ExampleHealthChecker;

#[async_trait::async_trait]
impl HealthCheckable for ExampleHealthChecker {
    async fn check_health(&self) -> HealthStatus {
        // 模拟健康检查
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // 随机返回健康状态（在实际应用中应该检查真实的健康状态）
        HealthStatus::healthy()
    }
    
    fn name(&self) -> &str {
        "ExampleHealthChecker"
    }
    
    fn timeout(&self) -> Duration {
        Duration::from_secs(5)
    }
}
