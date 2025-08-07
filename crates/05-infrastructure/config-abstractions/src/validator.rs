//! 配置验证抽象接口

use async_trait::async_trait;
use infrastructure_common::{ConfigError, ValidationError};
use serde_json::Value;
use std::collections::HashMap;

/// 配置验证器 trait
/// 
/// 定义配置验证的统一接口
#[async_trait]
pub trait ConfigValidator<T>: Send + Sync {
    /// 验证配置
    async fn validate(&self, config: &T) -> Result<(), ValidationError>;
    
    /// 获取验证器名称
    fn name(&self) -> &str;
    
    /// 获取验证器版本
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    /// 获取验证器描述
    fn description(&self) -> Option<&str> {
        None
    }
    
    /// 是否为异步验证器
    fn is_async(&self) -> bool {
        false
    }
}

/// 配置验证管理器 trait
#[async_trait]
pub trait ConfigValidationManager: Send + Sync {
    /// 注册验证器
    async fn register_validator<T>(&mut self, validator: Box<dyn ConfigValidator<T>>) -> Result<(), ConfigError>
    where
        T: 'static;
    
    /// 移除验证器
    async fn unregister_validator<T>(&mut self) -> Result<(), ConfigError>
    where
        T: 'static;
    
    /// 验证特定类型的配置
    async fn validate_options_type<T>(&self, config: &T) -> Result<ValidationResult, ConfigError>
    where
        T: 'static;
    
    /// 验证所有已注册的配置类型
    async fn validate_all(&self) -> Result<ValidationResult, ConfigError>;
    
    /// 注册所有验证器
    async fn register_all_validators(&mut self) -> Result<(), ConfigError>;
    
    /// 获取已注册的验证器列表
    fn get_registered_validators(&self) -> Vec<ValidatorInfo>;
}

/// 验证器信息
#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    /// 验证器名称
    pub name: String,
    /// 验证的类型名称
    pub target_type: String,
    /// 验证器版本
    pub version: String,
    /// 验证器描述
    pub description: Option<String>,
    /// 是否为异步验证器
    pub is_async: bool,
}

/// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否验证通过
    pub is_valid: bool,
    /// 验证错误列表
    pub errors: Vec<ValidationError>,
    /// 验证警告列表
    pub warnings: Vec<ValidationWarning>,
    /// 验证的配置项数量
    pub validated_count: usize,
    /// 验证耗时
    pub duration: std::time::Duration,
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
            validated_count: 0,
            duration: std::time::Duration::ZERO,
            validated_at: chrono::Utc::now(),
        }
    }
    
    /// 创建失败的验证结果
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
            validated_count: 0,
            duration: std::time::Duration::ZERO,
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
    
    /// 合并验证结果
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.validated_count += other.validated_count;
        
        if !other.is_valid {
            self.is_valid = false;
        }
    }
}

/// 验证警告
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// 警告字段
    pub field: String,
    /// 警告消息
    pub message: String,
    /// 建议修复方法
    pub suggestion: Option<String>,
}

impl ValidationWarning {
    /// 创建新的验证警告
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            suggestion: None,
        }
    }
    
    /// 添加建议
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// 通用配置验证器
#[derive(Debug)]
pub struct GenericConfigValidator {
    /// 验证规则
    pub rules: HashMap<String, ValidationRule>,
    /// 验证器名称
    pub name: String,
}

impl GenericConfigValidator {
    /// 创建新的通用验证器
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            rules: HashMap::new(),
            name: name.into(),
        }
    }
    
    /// 添加验证规则
    pub fn add_rule(&mut self, field: impl Into<String>, rule: ValidationRule) {
        self.rules.insert(field.into(), rule);
    }
    
    /// 验证 JSON 值
    pub async fn validate_value(&self, value: &Value) -> Result<ValidationResult, ConfigError> {
        let mut result = ValidationResult::success();
        
        for (field, rule) in &self.rules {
            if let Some(field_value) = value.get(field) {
                if let Err(error) = rule.validate(field, field_value) {
                    result.add_error(error);
                }
            } else if rule.required {
                result.add_error(ValidationError::required_field_missing(field));
            }
        }
        
        result.validated_count = self.rules.len();
        Ok(result)
    }
}

/// 验证规则
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// 是否必需
    pub required: bool,
    /// 数据类型
    pub data_type: Option<String>,
    /// 最小值
    pub min_value: Option<f64>,
    /// 最大值
    pub max_value: Option<f64>,
    /// 最小长度
    pub min_length: Option<usize>,
    /// 最大长度
    pub max_length: Option<usize>,
    /// 正则表达式模式
    pub pattern: Option<regex::Regex>,
    /// 枚举值
    pub enum_values: Option<Vec<String>>,
    /// 自定义验证函数
    pub custom_validator: Option<fn(&Value) -> Result<(), String>>,
}

impl ValidationRule {
    /// 创建新的验证规则
    pub fn new() -> Self {
        Self {
            required: false,
            data_type: None,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            custom_validator: None,
        }
    }
    
    /// 设置为必需字段
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
    
    /// 设置数据类型
    pub fn with_type(mut self, data_type: impl Into<String>) -> Self {
        self.data_type = Some(data_type.into());
        self
    }
    
    /// 设置值范围
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }
    
    /// 设置长度范围
    pub fn with_length_range(mut self, min: usize, max: usize) -> Self {
        self.min_length = Some(min);
        self.max_length = Some(max);
        self
    }
    
    /// 验证值
    pub fn validate(&self, field: &str, value: &Value) -> Result<(), ValidationError> {
        // 类型检查
        if let Some(expected_type) = &self.data_type {
            if !self.check_type(value, expected_type) {
                return Err(ValidationError::invalid_field_value(
                    field,
                    value.to_string(),
                    format!("期望类型: {}", expected_type),
                ));
            }
        }
        
        // 值范围检查
        if let Some(min) = self.min_value {
            if let Some(num) = value.as_f64() {
                if num < min {
                    return Err(ValidationError::invalid_field_value(
                        field,
                        num.to_string(),
                        format!("值不能小于 {}", min),
                    ));
                }
            }
        }
        
        if let Some(max) = self.max_value {
            if let Some(num) = value.as_f64() {
                if num > max {
                    return Err(ValidationError::invalid_field_value(
                        field,
                        num.to_string(),
                        format!("值不能大于 {}", max),
                    ));
                }
            }
        }
        
        // 长度检查
        if let Some(str_val) = value.as_str() {
            if let Some(min_len) = self.min_length {
                if str_val.len() < min_len {
                    return Err(ValidationError::invalid_field_value(
                        field,
                        str_val.to_string(),
                        format!("长度不能小于 {}", min_len),
                    ));
                }
            }
            
            if let Some(max_len) = self.max_length {
                if str_val.len() > max_len {
                    return Err(ValidationError::invalid_field_value(
                        field,
                        str_val.to_string(),
                        format!("长度不能大于 {}", max_len),
                    ));
                }
            }
        }
        
        // 枚举值检查
        if let Some(enum_values) = &self.enum_values {
            if let Some(str_val) = value.as_str() {
                if !enum_values.contains(&str_val.to_string()) {
                    return Err(ValidationError::invalid_field_value(
                        field,
                        str_val.to_string(),
                        format!("必须是以下值之一: {:?}", enum_values),
                    ));
                }
            }
        }
        
        // 自定义验证
        if let Some(validator) = self.custom_validator {
            if let Err(msg) = validator(value) {
                return Err(ValidationError::invalid_field_value(field, value.to_string(), msg));
            }
        }
        
        Ok(())
    }
    
    /// 检查类型
    fn check_type(&self, value: &Value, expected_type: &str) -> bool {
        match expected_type.to_lowercase().as_str() {
            "string" => value.is_string(),
            "number" | "integer" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => true, // 未知类型，跳过检查
        }
    }
}

impl Default for ValidationRule {
    fn default() -> Self {
        Self::new()
    }
}
