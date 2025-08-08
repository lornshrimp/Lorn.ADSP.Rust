//! 配置管理器抽象接口

use async_trait::async_trait;
use infrastructure_common::{ConfigError, ConfigSection, Configurable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::provider::ConfigProvider;
use crate::validator::ConfigValidator;
use std::any::TypeId;

/// 配置管理器 trait
/// 
/// 提供配置的统一管理接口，支持多个配置源
#[async_trait]
pub trait ConfigManager: Send + Sync {
    /// 注册配置提供者
    async fn register_provider(&mut self, provider: Box<dyn ConfigProvider>) -> Result<(), ConfigError>;
    
    /// 移除配置提供者
    async fn unregister_provider(&mut self, provider_name: &str) -> Result<(), ConfigError>;
    
    /// 获取配置值
    async fn get_configuration(&self, key: &str) -> Result<Value, ConfigError>;
    
    /// 获取配置节
    async fn get_section(&self, section_name: &str) -> Result<ConfigSection, ConfigError>;
    
    /// 绑定配置到指定类型
    async fn bind_configuration<T>(&self, key: &str) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static;
    
    /// 绑定配置到实例
    async fn bind_to_instance<T>(&self, instance: &mut T, path: &str) -> Result<(), ConfigError>
    where
        T: Configurable;
    
    /// 重新加载所有配置
    async fn reload_all(&mut self) -> Result<(), ConfigError>;
    
    /// 验证配置
    async fn validate_configuration(&self) -> Result<ValidationResult, ConfigError>;
    
    /// 注册配置验证器
    async fn register_validator<T>(&mut self, validator: Box<dyn ConfigValidator<T>>) -> Result<(), ConfigError>
    where
        T: 'static;
    
    /// 注册所有组件的配置选项
    async fn register_all_options(&mut self) -> Result<(), ConfigError>;
    
    /// 注册特定组件的配置选项
    async fn register_component_options<T>(&mut self) -> Result<(), ConfigError>
    where
        T: Configurable + 'static;
}

/// 类型化配置绑定器 trait
/// 
/// 提供强类型的配置绑定功能
#[async_trait]
pub trait TypedConfigBinder: Send + Sync {
    /// 绑定配置到指定类型
    async fn bind_configuration<T>(&self, path: &str) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static;
    
    /// 绑定配置到实例
    async fn bind_to_instance<T>(&self, instance: &mut T, path: &str) -> Result<(), ConfigError>
    where
        T: Configurable;
    
    /// 绑定配置并验证
    async fn bind_and_validate<T>(&self, path: &str) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static;
        
    /// 获取配置类型信息
    fn get_config_type_info<T>(&self) -> ConfigTypeInfo
    where
        T: 'static;
}

/// 配置验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 验证是否通过
    pub is_valid: bool,
    /// 错误信息
    pub errors: Vec<ValidationError>,
    /// 警告信息
    pub warnings: Vec<ValidationWarning>,
    /// 验证时间
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

impl ValidationResult {
    /// 创建成功的验证结果
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            validated_at: chrono::Utc::now(),
        }
    }
    
    /// 创建失败的验证结果
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
            validated_at: chrono::Utc::now(),
        }
    }
    
    /// 添加错误
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.is_valid = false;
    }
    
    /// 添加警告
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }
}

/// 验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// 配置路径
    pub path: String,
    /// 错误消息
    pub message: String,
    /// 错误类型
    pub error_type: ValidationErrorType,
    /// 期望值
    pub expected: Option<String>,
    /// 实际值
    pub actual: Option<String>,
}

/// 验证警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// 配置路径
    pub path: String,
    /// 警告消息
    pub message: String,
    /// 建议
    pub suggestion: Option<String>,
}

/// 验证错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    /// 必需字段缺失
    RequiredFieldMissing,
    /// 类型不匹配
    TypeMismatch,
    /// 值超出范围
    ValueOutOfRange,
    /// 格式错误
    FormatError,
    /// 自定义错误
    Custom,
}

/// 配置类型信息
#[derive(Debug, Clone)]
pub struct ConfigTypeInfo {
    /// 类型ID
    pub type_id: TypeId,
    /// 类型名称
    pub type_name: String,
    /// 配置路径
    pub config_path: String,
    /// 是否必需
    pub required: bool,
}

/// 配置选项描述符
#[derive(Debug, Clone)]
pub struct ConfigOptionDescriptor {
    /// 选项路径
    pub path: String,
    /// 选项类型
    pub option_type: String,
    /// 默认值
    pub default_value: Option<Value>,
    /// 描述
    pub description: Option<String>,
    /// 是否必需
    pub required: bool,
    /// 验证规则
    pub validation_rules: Vec<String>,
}
