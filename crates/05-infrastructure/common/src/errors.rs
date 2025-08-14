//! 错误类型定义

use thiserror::Error;

/// 配置错误类型
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("配置文件不存在: {path}")]
    FileNotFound { path: String },

    #[error("配置文件读取失败: {source}")]
    FileReadError {
        #[from]
        source: std::io::Error,
    },

    #[error("配置解析失败: {source}")]
    ParseError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("配置验证失败: {message}")]
    ValidationError { message: String },

    #[error("配置序列化失败: {source}")]
    SerializationError {
        #[from]
        source: serde_json::Error,
    },

    #[error("配置键不存在: {key}")]
    KeyNotFound { key: String },

    #[error("配置类型转换失败: {message}")]
    TypeConversionError { message: String },

    #[error("配置重载失败: {message}")]
    ReloadError { message: String },

    #[error("配置验证失败: {errors:?}")]
    ValidationFailed { errors: Vec<String> },

    #[error("没有可回滚的配置")]
    NoRollbackAvailable,

    #[error("配置版本不存在: {version}")]
    VersionNotFound { version: u64 },

    #[error("配置文件监控失败: {message}")]
    WatchError { message: String },

    #[error("配置热重载失败: {message}")]
    HotReloadError { message: String },
}

/// 依赖注入错误类型
#[derive(Error, Debug)]
pub enum DependencyError {
    #[error("组件未注册: {type_name}")]
    ComponentNotRegistered { type_name: String },

    #[error("组件创建失败: {type_name}, 原因: {source}")]
    ComponentCreationFailed {
        type_name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("循环依赖检测到: {dependency_chain}")]
    CircularDependency { dependency_chain: String },

    #[error("依赖解析失败: {type_name}, 原因: {message}")]
    DependencyResolutionFailed { type_name: String, message: String },

    #[error("作用域不匹配: 期望 {expected}, 实际 {actual}")]
    ScopeMismatch { expected: String, actual: String },

    #[error("组件生命周期管理失败: {message}")]
    LifecycleError { message: String },

    #[error("组件注册失败: {type_name}, 原因: {message}")]
    RegistrationError { type_name: String, message: String },
}

/// 组件错误类型
#[derive(Error, Debug)]
pub enum ComponentError {
    #[error("组件扫描失败: {message}")]
    ScanError { message: String },

    #[error("组件发现失败: {message}")]
    DiscoveryError { message: String },

    #[error("组件注册失败: {type_name}, 原因: {message}")]
    RegistrationError { type_name: String, message: String },

    #[error("组件元数据无效: {message}")]
    InvalidMetadata { message: String },

    #[error("组件工厂创建失败: {type_name}, 原因: {message}")]
    FactoryCreationError { type_name: String, message: String },

    #[error("检测到循环依赖: {cycle}")]
    CircularDependency { cycle: String },
}

impl ComponentError {
    /// 创建扫描错误
    pub fn scan_error(message: impl Into<String>) -> Self {
        Self::ScanError {
            message: message.into(),
        }
    }

    /// 创建发现错误
    pub fn discovery_error(message: impl Into<String>) -> Self {
        Self::DiscoveryError {
            message: message.into(),
        }
    }
}

/// 验证错误类型
#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    #[error("验证失败: {message}")]
    ValidationFailed { message: String },

    #[error("必需字段缺失: {field_name}")]
    RequiredFieldMissing { field_name: String },

    #[error("字段值无效: {field_name}, 值: {value}, 原因: {reason}")]
    InvalidFieldValue {
        field_name: String,
        value: String,
        reason: String,
    },

    #[error("字段值超出范围: {field_name}, 值: {value}, 范围: {range}")]
    ValueOutOfRange {
        field_name: String,
        value: String,
        range: String,
    },

    #[error("格式错误: {field_name}, 期望格式: {expected_format}")]
    FormatError {
        field_name: String,
        expected_format: String,
    },
}

impl ValidationError {
    /// 创建新的验证错误
    pub fn new(message: impl Into<String>) -> Self {
        Self::ValidationFailed {
            message: message.into(),
        }
    }

    /// 创建必需字段缺失错误
    pub fn required_field_missing(field_name: impl Into<String>) -> Self {
        Self::RequiredFieldMissing {
            field_name: field_name.into(),
        }
    }

    /// 创建字段值无效错误
    pub fn invalid_field_value(
        field_name: impl Into<String>,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidFieldValue {
            field_name: field_name.into(),
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// 创建值超出范围错误
    pub fn value_out_of_range(
        field_name: impl Into<String>,
        value: impl Into<String>,
        range: impl Into<String>,
    ) -> Self {
        Self::ValueOutOfRange {
            field_name: field_name.into(),
            value: value.into(),
            range: range.into(),
        }
    }

    /// 创建格式错误
    pub fn format_error(field_name: impl Into<String>, expected_format: impl Into<String>) -> Self {
        Self::FormatError {
            field_name: field_name.into(),
            expected_format: expected_format.into(),
        }
    }
}

/// 生命周期管理错误类型
#[derive(Error, Debug)]
pub enum LifecycleError {
    #[error("作用域创建失败: {message}")]
    ScopeCreationFailed { message: String },

    #[error("作用域销毁失败: {scope_id}, 原因: {message}")]
    ScopeDestructionFailed { scope_id: String, message: String },

    #[error("作用域不存在: {scope_id}")]
    ScopeNotFound { scope_id: String },

    #[error("生命周期管理失败: {message}")]
    LifecycleManagementFailed { message: String },
}

/// 基础设施错误类型
#[derive(Error, Debug)]
pub enum InfrastructureError {
    #[error("配置错误: {source}")]
    ConfigError {
        #[from]
        source: ConfigError,
    },

    #[error("依赖注入错误: {source}")]
    DependencyError {
        #[from]
        source: DependencyError,
    },

    #[error("验证错误: {source}")]
    ValidationError {
        #[from]
        source: ValidationError,
    },

    #[error("生命周期错误: {source}")]
    LifecycleError {
        #[from]
        source: LifecycleError,
    },

    #[error("基础设施启动失败: {message}")]
    BootstrapFailed { message: String },

    #[error("基础设施关闭失败: {message}")]
    ShutdownFailed { message: String },

    #[error("健康检查失败: {component_name}, 原因: {message}")]
    HealthCheckFailed {
        component_name: String,
        message: String,
    },
}

/// 结果类型别名
pub type ConfigResult<T> = Result<T, ConfigError>;
pub type DependencyResult<T> = Result<T, DependencyError>;
pub type ValidationResult<T> = Result<T, ValidationError>;
pub type LifecycleResult<T> = Result<T, LifecycleError>;
pub type InfrastructureResult<T> = Result<T, InfrastructureError>;
