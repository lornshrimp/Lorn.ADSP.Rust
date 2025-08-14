//! 基础设施构建器

use crate::infrastructure::AdSystemInfrastructure;
use config_abstractions::{ConfigManager, ConfigProvider};
use config_impl::providers::{
    EnvironmentConfigProviderImpl, JsonConfigProvider, TomlConfigProvider,
};
use di_abstractions::ComponentScanner;
use infrastructure_common::{HealthCheckable, InfrastructureError};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// 基础设施构建器
///
/// 使用建造者模式构建完整的基础设施实例
pub struct InfrastructureBuilder {
    /// 配置源列表
    config_sources: Vec<Box<dyn ConfigProvider>>,
    /// 组件扫描器列表
    component_scanners: Vec<Box<dyn ComponentScanner>>,
    /// 健康检查器列表
    health_checks: Vec<Box<dyn HealthCheckable>>,
    /// 是否启用配置热重载
    hot_reload_enabled: bool,
    /// 环境变量前缀
    env_prefix: Option<String>,
    /// 配置验证是否启用
    validation_enabled: bool,
    /// 是否启用日志初始化
    logging_enabled: bool,
    /// 日志配置
    logging_config: LoggingConfig,
}

impl InfrastructureBuilder {
    /// 创建新的基础设施构建器
    pub fn new() -> Self {
        Self {
            config_sources: Vec::new(),
            component_scanners: Vec::new(),
            health_checks: Vec::new(),
            hot_reload_enabled: false,
            env_prefix: None,
            validation_enabled: true,
            logging_enabled: false, // 默认不启用日志初始化
            logging_config: LoggingConfig::default(),
        }
    }

    /// 添加 TOML 配置文件
    pub fn add_config_toml<P: AsRef<Path>>(mut self, path: P) -> Result<Self, InfrastructureError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(InfrastructureError::BootstrapFailed {
                message: format!("配置文件不存在: {}", path.display()),
            });
        }

        info!("添加 TOML 配置文件: {}", path.display());
        let provider = TomlConfigProvider::new(path)?;
        self.config_sources.push(Box::new(provider));
        Ok(self)
    }

    /// 添加 JSON 配置文件
    pub fn add_config_json<P: AsRef<Path>>(mut self, path: P) -> Result<Self, InfrastructureError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(InfrastructureError::BootstrapFailed {
                message: format!("配置文件不存在: {}", path.display()),
            });
        }

        info!("添加 JSON 配置文件: {}", path.display());
        let provider = JsonConfigProvider::new(path)?;
        self.config_sources.push(Box::new(provider));
        Ok(self)
    }

    /// 添加环境变量配置源
    pub fn add_config_env_vars<S: Into<String>>(
        mut self,
        prefix: S,
    ) -> Result<Self, InfrastructureError> {
        let prefix = prefix.into();
        info!("添加环境变量配置源，前缀: {}", prefix);

        let provider = EnvironmentConfigProviderImpl::new(prefix.clone())?;
        self.config_sources.push(Box::new(provider));
        self.env_prefix = Some(prefix);
        Ok(self)
    }

    /// 添加自定义配置提供者
    pub fn add_config_provider<T: ConfigProvider + 'static>(mut self, provider: T) -> Self {
        info!("添加自定义配置提供者: {}", provider.name());
        self.config_sources.push(Box::new(provider));
        self
    }

    /// 启用配置热重载
    pub fn enable_hot_reload(mut self, enabled: bool) -> Self {
        self.hot_reload_enabled = enabled;
        if enabled {
            info!("启用配置热重载");
        }
        self
    }

    /// 添加组件扫描器
    pub fn add_component_scanner<T: ComponentScanner + 'static>(mut self, scanner: T) -> Self {
        debug!("添加组件扫描器");
        self.component_scanners.push(Box::new(scanner));
        self
    }

    /// 扫描指定的 crate
    pub fn scan_crate<S: Into<String>>(self, crate_name: S) -> Result<Self, InfrastructureError> {
        let crate_name = crate_name.into();
        info!("添加 crate 扫描: {}", crate_name);

        // 这里应该实现实际的 crate 扫描器
        // 暂时使用占位实现

        Ok(self)
    }

    /// 添加健康检查器
    pub fn add_health_check<T: HealthCheckable + 'static>(mut self, checker: T) -> Self {
        info!("添加健康检查器: {}", checker.name());
        self.health_checks.push(Box::new(checker));
        self
    }

    /// 启用扩展配置源管理
    pub fn with_extended_config_sources(self) -> Self {
        info!("启用扩展配置源管理");
        // 在未来的版本中，这里会集成 ExtendedConfigSourceManager
        self
    }

    /// 启用高级组件扫描
    pub fn with_advanced_component_scanning(self) -> Self {
        info!("启用高级组件扫描");
        // 在未来的版本中，这里会集成 ComponentScannerImpl
        self
    }

    /// 自动配置开发环境
    pub fn auto_configure_development(mut self) -> Self {
        info!("自动配置开发环境");

        // 添加开发环境特定的配置
        if Path::new("./config.dev.toml").exists() {
            if let Ok(provider) = TomlConfigProvider::new("./config.dev.toml") {
                self.config_sources.push(Box::new(provider));
                debug!("添加开发环境配置: config.dev.toml");
            }
        }

        if Path::new("./appsettings.Development.json").exists() {
            if let Ok(provider) = JsonConfigProvider::new("./appsettings.Development.json") {
                self.config_sources.push(Box::new(provider));
                debug!("添加开发环境配置: appsettings.Development.json");
            }
        }

        // 启用热重载
        self.hot_reload_enabled = true;

        self
    }

    /// 自动配置生产环境
    pub fn auto_configure_production(mut self) -> Self {
        info!("自动配置生产环境");

        // 添加生产环境特定的配置
        if Path::new("./config.prod.toml").exists() {
            if let Ok(provider) = TomlConfigProvider::new("./config.prod.toml") {
                self.config_sources.push(Box::new(provider));
                debug!("添加生产环境配置: config.prod.toml");
            }
        }

        if Path::new("./appsettings.Production.json").exists() {
            if let Ok(provider) = JsonConfigProvider::new("./appsettings.Production.json") {
                self.config_sources.push(Box::new(provider));
                debug!("添加生产环境配置: appsettings.Production.json");
            }
        }

        // 生产环境通常不启用热重载
        self.hot_reload_enabled = false;

        self
    }
    pub fn enable_health_checks(self, enabled: bool) -> Self {
        if enabled {
            info!("启用健康检查");
            // 添加默认的健康检查器
        }
        self
    }

    /// 启用或禁用配置验证
    pub fn enable_validation(mut self, enabled: bool) -> Self {
        self.validation_enabled = enabled;
        self
    }

    /// 配置日志
    pub fn with_logging(mut self, config: LoggingConfig) -> Self {
        self.logging_config = config;
        self.logging_enabled = true; // 启用日志初始化
        self
    }

    /// 构建基础设施实例
    pub async fn build(self) -> Result<AdSystemInfrastructure, InfrastructureError> {
        info!("开始构建基础设施");

        // 只有在明确配置了日志时才初始化日志
        // 避免在测试环境中重复初始化
        if self.logging_enabled {
            self.initialize_logging()?;
        }

        // 创建配置管理器
        let mut config_manager = config_impl::manager::AdSystemConfigManager::new();

        // 注册所有配置提供者
        for provider in self.config_sources {
            config_manager
                .register_provider(provider)
                .await
                .map_err(|e| InfrastructureError::BootstrapFailed {
                    message: format!("注册配置提供者失败: {}", e),
                })?;
        }

        // 如果启用热重载，配置热重载功能
        if self.hot_reload_enabled {
            info!("启用配置热重载");

            // 创建文件监控器
            let file_watcher = config_impl::watcher::ConfigFileWatcher::new().map_err(|e| {
                InfrastructureError::BootstrapFailed {
                    message: format!("创建文件监控器失败: {}", e),
                }
            })?;
            let watcher = Arc::new(tokio::sync::Mutex::new(file_watcher));

            config_manager
                .enable_hot_reload(watcher)
                .await
                .map_err(|e| InfrastructureError::BootstrapFailed {
                    message: format!("启用热重载失败: {}", e),
                })?;
        }

        // 如果启用验证，进行配置验证
        if self.validation_enabled {
            info!("开始配置验证");
            let validation_result = config_manager.validate_configuration().await.map_err(|e| {
                InfrastructureError::BootstrapFailed {
                    message: format!("配置验证失败: {}", e),
                }
            })?;

            if !validation_result.is_valid {
                let error_messages: Vec<String> = validation_result
                    .errors
                    .iter()
                    .map(|e| format!("{}: {}", e.path, e.message))
                    .collect();
                return Err(InfrastructureError::BootstrapFailed {
                    message: format!("配置验证失败: {}", error_messages.join(", ")),
                });
            }

            if !validation_result.warnings.is_empty() {
                for warning in validation_result.warnings {
                    tracing::warn!("配置警告 [{}]: {}", warning.path, warning.message);
                }
            }
        }

        let config_manager = Arc::new(config_manager);

        // 创建依赖注入容器
        let di_container = di_impl::DiContainerImpl::new();

        // 创建基础设施实例
        let infrastructure =
            AdSystemInfrastructure::new(config_manager, di_container, self.health_checks);

        info!("基础设施构建完成");
        Ok(infrastructure)
    }

    /// 初始化日志系统
    fn initialize_logging(&self) -> Result<(), InfrastructureError> {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(self.logging_config.level)
            .with_target(self.logging_config.show_target)
            .with_thread_ids(self.logging_config.show_thread_ids)
            .with_file(self.logging_config.show_file)
            .with_line_number(self.logging_config.show_line_number);

        if self.logging_config.json_format {
            subscriber.json().try_init()
        } else {
            subscriber.try_init()
        }
        .map_err(|e| InfrastructureError::BootstrapFailed {
            message: format!("日志初始化失败: {}", e),
        })?;

        info!("日志系统初始化完成");
        Ok(())
    }
}

impl Default for InfrastructureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 日志配置
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: tracing::Level,
    /// 是否显示目标
    pub show_target: bool,
    /// 是否显示线程ID
    pub show_thread_ids: bool,
    /// 是否显示文件名
    pub show_file: bool,
    /// 是否显示行号
    pub show_line_number: bool,
    /// 是否使用 JSON 格式
    pub json_format: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: tracing::Level::INFO,
            show_target: true,
            show_thread_ids: false,
            show_file: false,
            show_line_number: false,
            json_format: false,
        }
    }
}

impl LoggingConfig {
    /// 创建开发环境日志配置
    pub fn development() -> Self {
        Self {
            level: tracing::Level::DEBUG,
            show_target: true,
            show_thread_ids: true,
            show_file: true,
            show_line_number: true,
            json_format: false,
        }
    }

    /// 创建生产环境日志配置
    pub fn production() -> Self {
        Self {
            level: tracing::Level::INFO,
            show_target: false,
            show_thread_ids: false,
            show_file: false,
            show_line_number: false,
            json_format: true,
        }
    }
}
