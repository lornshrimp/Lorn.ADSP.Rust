//! 扩展的配置源支持
//! 
//! 提供多种配置源的统一管理和集成功能

use config_abstractions::ConfigProvider;
use config_impl::providers::{
    TomlConfigProvider, JsonConfigProvider, EnvironmentConfigProviderImpl,
};
use infrastructure_common::{InfrastructureError, ConfigError};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::{info, debug, warn};

/// 配置源类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSourceType {
    /// TOML 文件
    Toml,
    /// JSON 文件
    Json,
    /// YAML 文件
    Yaml,
    /// 环境变量
    Environment,
    /// 远程配置
    Remote,
    /// 数据库配置
    Database,
    /// 内存配置
    Memory,
}

/// 配置源描述
#[derive(Debug, Clone)]
pub struct ConfigSourceDescriptor {
    /// 配置源类型
    pub source_type: ConfigSourceType,
    /// 配置路径或标识符
    pub location: String,
    /// 优先级（数字越小优先级越高）
    pub priority: u32,
    /// 是否启用热重载
    pub hot_reload: bool,
    /// 附加选项
    pub options: ConfigSourceOptions,
}

/// 配置源选项
#[derive(Debug, Clone)]
pub struct ConfigSourceOptions {
    /// 环境变量前缀
    pub env_prefix: Option<String>,
    /// 远程配置的认证信息
    pub auth_token: Option<String>,
    /// 数据库连接字符串
    pub connection_string: Option<String>,
    /// 配置节点路径
    pub section_path: Option<String>,
    /// 超时设置（毫秒）
    pub timeout_ms: Option<u64>,
    /// 重试次数
    pub retry_count: Option<u32>,
}

impl Default for ConfigSourceOptions {
    fn default() -> Self {
        Self {
            env_prefix: None,
            auth_token: None,
            connection_string: None,
            section_path: None,
            timeout_ms: Some(5000), // 默认5秒超时
            retry_count: Some(3),   // 默认重试3次
        }
    }
}

/// 扩展配置源管理器
/// 
/// 统一管理多种类型的配置源，支持优先级排序和热重载
pub struct ExtendedConfigSourceManager {
    /// 配置源描述列表
    sources: Vec<ConfigSourceDescriptor>,
    /// 已创建的配置提供者
    providers: Vec<Box<dyn ConfigProvider>>,
}

impl ExtendedConfigSourceManager {
    /// 创建新的配置源管理器
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            providers: Vec::new(),
        }
    }
    
    /// 添加 TOML 文件配置源
    pub fn add_toml_file<P: AsRef<Path>>(
        mut self,
        path: P,
        priority: u32,
        hot_reload: bool,
    ) -> Self {
        let descriptor = ConfigSourceDescriptor {
            source_type: ConfigSourceType::Toml,
            location: path.as_ref().to_string_lossy().to_string(),
            priority,
            hot_reload,
            options: ConfigSourceOptions::default(),
        };
        
        self.sources.push(descriptor);
        debug!("添加 TOML 配置源: {}", path.as_ref().display());
        self
    }
    
    /// 添加 JSON 文件配置源
    pub fn add_json_file<P: AsRef<Path>>(
        mut self,
        path: P,
        priority: u32,
        hot_reload: bool,
    ) -> Self {
        let descriptor = ConfigSourceDescriptor {
            source_type: ConfigSourceType::Json,
            location: path.as_ref().to_string_lossy().to_string(),
            priority,
            hot_reload,
            options: ConfigSourceOptions::default(),
        };
        
        self.sources.push(descriptor);
        debug!("添加 JSON 配置源: {}", path.as_ref().display());
        self
    }
    
    /// 添加 YAML 文件配置源
    pub fn add_yaml_file<P: AsRef<Path>>(
        mut self,
        path: P,
        priority: u32,
        hot_reload: bool,
    ) -> Self {
        let descriptor = ConfigSourceDescriptor {
            source_type: ConfigSourceType::Yaml,
            location: path.as_ref().to_string_lossy().to_string(),
            priority,
            hot_reload,
            options: ConfigSourceOptions::default(),
        };
        
        self.sources.push(descriptor);
        debug!("添加 YAML 配置源: {}", path.as_ref().display());
        self
    }
    
    /// 添加环境变量配置源
    pub fn add_environment(
        mut self,
        prefix: Option<String>,
        priority: u32,
    ) -> Self {
        let mut options = ConfigSourceOptions::default();
        options.env_prefix = prefix.clone();
        
        let descriptor = ConfigSourceDescriptor {
            source_type: ConfigSourceType::Environment,
            location: prefix.unwrap_or_else(|| "ENV".to_string()),
            priority,
            hot_reload: false, // 环境变量通常不支持热重载
            options,
        };
        
        self.sources.push(descriptor.clone());
        debug!("添加环境变量配置源: {}", descriptor.location);
        self
    }
    
    /// 添加远程配置源
    pub fn add_remote_source(
        mut self,
        url: String,
        auth_token: Option<String>,
        priority: u32,
        hot_reload: bool,
    ) -> Self {
        let mut options = ConfigSourceOptions::default();
        options.auth_token = auth_token;
        
        let descriptor = ConfigSourceDescriptor {
            source_type: ConfigSourceType::Remote,
            location: url,
            priority,
            hot_reload,
            options,
        };
        
        self.sources.push(descriptor.clone());
        debug!("添加远程配置源: {}", descriptor.location);
        self
    }
    
    /// 添加数据库配置源
    pub fn add_database_source(
        mut self,
        connection_string: String,
        table_or_collection: String,
        priority: u32,
    ) -> Self {
        let mut options = ConfigSourceOptions::default();
        options.connection_string = Some(connection_string);
        options.section_path = Some(table_or_collection.clone());
        
        let descriptor = ConfigSourceDescriptor {
            source_type: ConfigSourceType::Database,
            location: format!("db://{}", table_or_collection),
            priority,
            hot_reload: false, // 数据库配置通常需要手动刷新
            options,
        };
        
        self.sources.push(descriptor.clone());
        debug!("添加数据库配置源: {}", descriptor.location);
        self
    }
    
    /// 构建配置提供者列表
    pub async fn build_providers(mut self) -> Result<Vec<Box<dyn ConfigProvider>>, InfrastructureError> {
        info!("开始构建配置提供者，共有 {} 个配置源", self.sources.len());
        
        // 按优先级排序
        self.sources.sort_by_key(|source| source.priority);
        
        let mut providers: Vec<Box<dyn ConfigProvider>> = Vec::new();
        
        for source in &self.sources {
            match self.create_provider(source).await {
                Ok(provider) => {
                    info!(
                        "成功创建配置提供者: {} (优先级: {})",
                        source.location,
                        source.priority
                    );
                    providers.push(provider);
                }
                Err(e) => {
                    warn!(
                        "创建配置提供者失败: {} - {:?}",
                        source.location,
                        e
                    );
                    // 根据配置决定是否继续或失败
                    // 这里选择继续，只记录警告
                }
            }
        }
        
        if providers.is_empty() {
            return Err(InfrastructureError::ConfigError {
                source: ConfigError::ValidationError {
                    message: "没有可用的配置提供者".to_string(),
                },
            });
        }
        
        info!("配置提供者构建完成，共 {} 个有效提供者", providers.len());
        Ok(providers)
    }
    
    /// 创建单个配置提供者
    async fn create_provider(
        &self,
        descriptor: &ConfigSourceDescriptor,
    ) -> Result<Box<dyn ConfigProvider>, InfrastructureError> {
        match descriptor.source_type {
            ConfigSourceType::Toml => {
                // 检查文件是否存在
                if !Path::new(&descriptor.location).exists() {
                    return Err(InfrastructureError::ConfigError {
                        source: ConfigError::FileNotFound {
                            path: descriptor.location.clone(),
                        },
                    });
                }
                
                let provider = TomlConfigProvider::new(&descriptor.location)?;
                Ok(Box::new(provider))
            }
            
            ConfigSourceType::Json => {
                if !Path::new(&descriptor.location).exists() {
                    return Err(InfrastructureError::ConfigError {
                        source: ConfigError::FileNotFound {
                            path: descriptor.location.clone(),
                        },
                    });
                }
                
                let provider = JsonConfigProvider::new(&descriptor.location)?;
                Ok(Box::new(provider))
            }
            
            ConfigSourceType::Environment => {
                let prefix = descriptor.options.env_prefix.clone();
                let provider = if let Some(prefix) = prefix {
                    EnvironmentConfigProviderImpl::new(prefix)?
                } else {
                    EnvironmentConfigProviderImpl::new("ADSP_")?
                };
                Ok(Box::new(provider))
            }
            
            // 未实现的配置源类型
            ConfigSourceType::Yaml => {
                Err(InfrastructureError::ConfigError {
                    source: ConfigError::ValidationError {
                        message: "YAML 配置提供者尚未实现".to_string(),
                    },
                })
            }
            
            ConfigSourceType::Remote => {
                Err(InfrastructureError::ConfigError {
                    source: ConfigError::ValidationError {
                        message: "远程配置提供者尚未实现".to_string(),
                    },
                })
            }
            
            ConfigSourceType::Database => {
                Err(InfrastructureError::ConfigError {
                    source: ConfigError::ValidationError {
                        message: "数据库配置提供者尚未实现".to_string(),
                    },
                })
            }
            
            ConfigSourceType::Memory => {
                Err(InfrastructureError::ConfigError {
                    source: ConfigError::ValidationError {
                        message: "内存配置提供者尚未实现".to_string(),
                    },
                })
            }
        }
    }
    
    /// 获取配置源描述列表
    pub fn get_sources(&self) -> &[ConfigSourceDescriptor] {
        &self.sources
    }
    
    /// 验证所有配置源的可用性
    pub async fn validate_sources(&self) -> Result<(), InfrastructureError> {
        info!("开始验证配置源可用性");
        
        for source in &self.sources {
            match source.source_type {
                ConfigSourceType::Toml | ConfigSourceType::Json | ConfigSourceType::Yaml => {
                    // 检查文件是否存在且可读
                    let path = Path::new(&source.location);
                    if !path.exists() {
                        warn!("配置文件不存在: {}", source.location);
                        continue;
                    }
                    
                    match fs::metadata(path).await {
                        Ok(metadata) => {
                            if !metadata.is_file() {
                                warn!("配置路径不是文件: {}", source.location);
                            } else {
                                debug!("配置文件验证通过: {}", source.location);
                            }
                        }
                        Err(e) => {
                            warn!("配置文件访问失败: {} - {:?}", source.location, e);
                        }
                    }
                }
                
                ConfigSourceType::Environment => {
                    debug!("环境变量配置源总是可用");
                }
                
                ConfigSourceType::Remote => {
                    // TODO: 实现远程配置验证
                    debug!("远程配置源验证跳过（未实现）");
                }
                
                ConfigSourceType::Database => {
                    // TODO: 实现数据库配置验证
                    debug!("数据库配置源验证跳过（未实现）");
                }
                
                ConfigSourceType::Memory => {
                    debug!("内存配置源总是可用");
                }
            }
        }
        
        info!("配置源验证完成");
        Ok(())
    }
}

impl Default for ExtendedConfigSourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 配置源管理器构建器
/// 
/// 提供更便捷的配置源配置方式
pub struct ConfigSourceManagerBuilder {
    manager: ExtendedConfigSourceManager,
}

impl ConfigSourceManagerBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            manager: ExtendedConfigSourceManager::new(),
        }
    }
    
    /// 从默认位置自动添加配置文件
    pub fn auto_discover(mut self) -> Self {
        // 尝试添加常见的配置文件位置
        let common_paths = [
            ("./config.toml", ConfigSourceType::Toml),
            ("./config.json", ConfigSourceType::Json),
            ("./appsettings.json", ConfigSourceType::Json),
            ("./application.toml", ConfigSourceType::Toml),
        ];
        
        for (path, source_type) in &common_paths {
            if Path::new(path).exists() {
                match source_type {
                    ConfigSourceType::Toml => {
                        self.manager = self.manager.add_toml_file(path, 100, true);
                    }
                    ConfigSourceType::Json => {
                        self.manager = self.manager.add_json_file(path, 100, true);
                    }
                    _ => {}
                }
                info!("自动发现配置文件: {}", path);
            }
        }
        
        // 默认添加环境变量
        self.manager = self.manager.add_environment(Some("ADSP_".to_string()), 50);
        
        self
    }
    
    /// 添加开发环境配置
    pub fn add_development_config(mut self) -> Self {
        // 开发环境特定的配置文件
        let dev_paths = [
            "./config.dev.toml",
            "./config.development.json",
            "./appsettings.Development.json",
        ];
        
        for path in &dev_paths {
            if Path::new(path).exists() {
                if path.ends_with(".toml") {
                    self.manager = self.manager.add_toml_file(path, 80, true);
                } else if path.ends_with(".json") {
                    self.manager = self.manager.add_json_file(path, 80, true);
                }
                info!("添加开发环境配置: {}", path);
            }
        }
        
        self
    }
    
    /// 添加生产环境配置
    pub fn add_production_config(mut self) -> Self {
        // 生产环境特定的配置文件
        let prod_paths = [
            "./config.prod.toml",
            "./config.production.json",
            "./appsettings.Production.json",
        ];
        
        for path in &prod_paths {
            if Path::new(path).exists() {
                if path.ends_with(".toml") {
                    self.manager = self.manager.add_toml_file(path, 70, true);
                } else if path.ends_with(".json") {
                    self.manager = self.manager.add_json_file(path, 70, true);
                }
                info!("添加生产环境配置: {}", path);
            }
        }
        
        self
    }
    
    /// 构建配置源管理器
    pub fn build(self) -> ExtendedConfigSourceManager {
        info!("配置源管理器构建完成，共 {} 个配置源", self.manager.sources.len());
        self.manager
    }
}

impl Default for ConfigSourceManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
