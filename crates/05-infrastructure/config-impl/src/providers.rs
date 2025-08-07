//! 配置提供者实现

use async_trait::async_trait;
use config_abstractions::{ConfigProvider, FileConfigProvider, EnvironmentConfigProvider as EnvironmentConfigProviderTrait};
use infrastructure_common::{ConfigError, ConfigSection};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, error, warn};

/// TOML 配置提供者
#[derive(Debug)]
pub struct TomlConfigProvider {
    file_path: PathBuf,
    config: Option<toml::Value>,
    last_modified: Option<SystemTime>,
    priority: i32,
}

impl TomlConfigProvider {
    /// 创建新的 TOML 配置提供者
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let file_path = path.as_ref().to_path_buf();
        let mut provider = Self {
            file_path,
            config: None,
            last_modified: None,
            priority: 100, // TOML 文件默认高优先级
        };
        
        // 初始加载
        provider.load_config()?;
        Ok(provider)
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// 加载配置文件
    fn load_config(&mut self) -> Result<(), ConfigError> {
        debug!("加载 TOML 配置文件: {}", self.file_path.display());
        
        let content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| ConfigError::FileReadError { source: e })?;
        
        self.config = Some(toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            source: Box::new(e),
        })?);
        
        self.last_modified = Some(
            std::fs::metadata(&self.file_path)
                .and_then(|m| m.modified())
                .map_err(|e| ConfigError::FileReadError { source: e })?,
        );
        
        debug!("TOML 配置文件加载完成");
        Ok(())
    }
    
    /// 将 TOML 值转换为 JSON 值
    fn toml_to_json(&self, value: &toml::Value) -> Value {
        match value {
            toml::Value::String(s) => Value::String(s.clone()),
            toml::Value::Integer(i) => Value::Number(serde_json::Number::from(*i)),
            toml::Value::Float(f) => Value::Number(
                serde_json::Number::from_f64(*f).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
            toml::Value::Boolean(b) => Value::Bool(*b),
            toml::Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.toml_to_json(v)).collect())
            }
            toml::Value::Table(table) => Value::Object(
                table
                    .iter()
                    .map(|(k, v)| (k.clone(), self.toml_to_json(v)))
                    .collect(),
            ),
            toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        }
    }
    
    /// 从嵌套路径获取值
    fn get_nested_value(&self, path: &str) -> Option<&toml::Value> {
        let config = self.config.as_ref()?;
        let parts: Vec<&str> = path.split('.').collect();
        
        let mut current = config;
        for part in parts {
            match current {
                toml::Value::Table(table) => {
                    current = table.get(part)?;
                }
                _ => return None,
            }
        }
        
        Some(current)
    }
}

#[async_trait]
impl ConfigProvider for TomlConfigProvider {
    async fn get_configuration(&self, key: &str) -> Result<Value, ConfigError> {
        match self.get_nested_value(key) {
            Some(value) => Ok(self.toml_to_json(value)),
            None => Err(ConfigError::KeyNotFound { key: key.to_string() }),
        }
    }
    
    async fn get_section(&self, section_name: &str) -> Result<ConfigSection, ConfigError> {
        match self.get_nested_value(section_name) {
            Some(toml::Value::Table(table)) => {
                let mut section = ConfigSection::new();
                for (key, value) in table {
                    section.insert(key.clone(), self.toml_to_json(value));
                }
                Ok(section)
            }
            Some(_) => Err(ConfigError::TypeConversionError {
                message: format!("配置节 {} 不是表类型", section_name),
            }),
            None => Err(ConfigError::KeyNotFound {
                key: section_name.to_string(),
            }),
        }
    }
    
    async fn reload(&mut self) -> Result<(), ConfigError> {
        self.load_config()
    }
    
    async fn contains_key(&self, key: &str) -> Result<bool, ConfigError> {
        Ok(self.get_nested_value(key).is_some())
    }
    
    async fn get_all_keys(&self) -> Result<Vec<String>, ConfigError> {
        let mut keys = Vec::new();
        if let Some(toml::Value::Table(table)) = &self.config {
            self.collect_keys(table, String::new(), &mut keys);
        }
        Ok(keys)
    }
    
    fn name(&self) -> &str {
        "TomlConfigProvider"
    }
    
    fn priority(&self) -> i32 {
        self.priority
    }
    
    fn supports_hot_reload(&self) -> bool {
        true
    }
}

impl TomlConfigProvider {
    /// 递归收集所有键
    fn collect_keys(&self, table: &toml::Table, prefix: String, keys: &mut Vec<String>) {
        for (key, value) in table {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };
            
            keys.push(full_key.clone());
            
            if let toml::Value::Table(nested_table) = value {
                self.collect_keys(nested_table, full_key, keys);
            }
        }
    }
}

#[async_trait]
impl FileConfigProvider for TomlConfigProvider {
    fn file_path(&self) -> &str {
        self.file_path.to_str().unwrap_or("unknown")
    }
    
    async fn file_exists(&self) -> bool {
        self.file_path.exists()
    }
    
    async fn last_modified(&self) -> Result<SystemTime, ConfigError> {
        self.last_modified.ok_or_else(|| ConfigError::ValidationError {
            message: "文件尚未加载".to_string(),
        })
    }
}

/// JSON 配置提供者
#[derive(Debug)]
pub struct JsonConfigProvider {
    file_path: PathBuf,
    config: Option<Value>,
    last_modified: Option<SystemTime>,
    priority: i32,
}

impl JsonConfigProvider {
    /// 创建新的 JSON 配置提供者
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let file_path = path.as_ref().to_path_buf();
        let mut provider = Self {
            file_path,
            config: None,
            last_modified: None,
            priority: 90, // JSON 文件中等优先级
        };
        
        provider.load_config()?;
        Ok(provider)
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// 加载配置文件
    fn load_config(&mut self) -> Result<(), ConfigError> {
        debug!("加载 JSON 配置文件: {}", self.file_path.display());
        
        let content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| ConfigError::FileReadError { source: e })?;
        
        self.config = Some(serde_json::from_str(&content)?);
        
        self.last_modified = Some(
            std::fs::metadata(&self.file_path)
                .and_then(|m| m.modified())
                .map_err(|e| ConfigError::FileReadError { source: e })?,
        );
        
        debug!("JSON 配置文件加载完成");
        Ok(())
    }
    
    /// 从嵌套路径获取值
    fn get_nested_value(&self, path: &str) -> Option<&Value> {
        let config = self.config.as_ref()?;
        let parts: Vec<&str> = path.split('.').collect();
        
        let mut current = config;
        for part in parts {
            current = current.get(part)?;
        }
        
        Some(current)
    }
}

#[async_trait]
impl ConfigProvider for JsonConfigProvider {
    async fn get_configuration(&self, key: &str) -> Result<Value, ConfigError> {
        match self.get_nested_value(key) {
            Some(value) => Ok(value.clone()),
            None => Err(ConfigError::KeyNotFound { key: key.to_string() }),
        }
    }
    
    async fn get_section(&self, section_name: &str) -> Result<ConfigSection, ConfigError> {
        match self.get_nested_value(section_name) {
            Some(Value::Object(obj)) => {
                let mut section = ConfigSection::new();
                for (key, value) in obj {
                    section.insert(key.clone(), value.clone());
                }
                Ok(section)
            }
            Some(_) => Err(ConfigError::TypeConversionError {
                message: format!("配置节 {} 不是对象类型", section_name),
            }),
            None => Err(ConfigError::KeyNotFound {
                key: section_name.to_string(),
            }),
        }
    }
    
    async fn reload(&mut self) -> Result<(), ConfigError> {
        self.load_config()
    }
    
    async fn contains_key(&self, key: &str) -> Result<bool, ConfigError> {
        Ok(self.get_nested_value(key).is_some())
    }
    
    async fn get_all_keys(&self) -> Result<Vec<String>, ConfigError> {
        let mut keys = Vec::new();
        if let Some(Value::Object(obj)) = &self.config {
            self.collect_keys(obj, String::new(), &mut keys);
        }
        Ok(keys)
    }
    
    fn name(&self) -> &str {
        "JsonConfigProvider"
    }
    
    fn priority(&self) -> i32 {
        self.priority
    }
    
    fn supports_hot_reload(&self) -> bool {
        true
    }
}

impl JsonConfigProvider {
    /// 递归收集所有键
    fn collect_keys(&self, obj: &serde_json::Map<String, Value>, prefix: String, keys: &mut Vec<String>) {
        for (key, value) in obj {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };
            
            keys.push(full_key.clone());
            
            if let Value::Object(nested_obj) = value {
                self.collect_keys(nested_obj, full_key, keys);
            }
        }
    }
}

#[async_trait]
impl FileConfigProvider for JsonConfigProvider {
    fn file_path(&self) -> &str {
        self.file_path.to_str().unwrap_or("unknown")
    }
    
    async fn file_exists(&self) -> bool {
        self.file_path.exists()
    }
    
    async fn last_modified(&self) -> Result<SystemTime, ConfigError> {
        self.last_modified.ok_or_else(|| ConfigError::ValidationError {
            message: "文件尚未加载".to_string(),
        })
    }
}

/// 环境变量配置提供者
#[derive(Debug)]
pub struct EnvironmentConfigProviderImpl {
    prefix: String,
    separator: String,
    priority: i32,
    env_vars: HashMap<String, String>,
}

impl EnvironmentConfigProviderImpl {
    /// 创建新的环境变量配置提供者
    pub fn new(prefix: impl Into<String>) -> Result<Self, ConfigError> {
        let prefix = prefix.into();
        let mut provider = Self {
            prefix,
            separator: "_".to_string(),
            priority: 200, // 环境变量最高优先级
            env_vars: HashMap::new(),
        };
        
        provider.load_env_vars()?;
        Ok(provider)
    }
    
    /// 设置分隔符
    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// 加载环境变量
    fn load_env_vars(&mut self) -> Result<(), ConfigError> {
        debug!("加载环境变量，前缀: {}", self.prefix);
        
        self.env_vars.clear();
        
        for (key, value) in std::env::vars() {
            if key.starts_with(&self.prefix) {
                let config_key = self.env_key_to_config_key(&key);
                self.env_vars.insert(config_key, value);
            }
        }
        
        debug!("加载了 {} 个环境变量", self.env_vars.len());
        Ok(())
    }
    
    /// 将环境变量键转换为配置键
    fn env_key_to_config_key(&self, env_key: &str) -> String {
        let key = env_key
            .strip_prefix(&self.prefix)
            .unwrap_or(env_key)
            .trim_start_matches(&self.separator);
        
        // 将下划线转换为点分隔符，并转换为小写
        key.replace(&self.separator, ".").to_lowercase()
    }
    
    /// 将配置键转换为环境变量键
    fn config_key_to_env_key(&self, config_key: &str) -> String {
        format!(
            "{}{}{}",
            self.prefix,
            self.separator,
            config_key.replace('.', &self.separator).to_uppercase()
        )
    }
}

#[async_trait]
impl ConfigProvider for EnvironmentConfigProviderImpl {
    async fn get_configuration(&self, key: &str) -> Result<Value, ConfigError> {
        match self.env_vars.get(key) {
            Some(value) => {
                // 尝试解析为不同类型
                if let Ok(bool_val) = value.parse::<bool>() {
                    Ok(Value::Bool(bool_val))
                } else if let Ok(int_val) = value.parse::<i64>() {
                    Ok(Value::Number(serde_json::Number::from(int_val)))
                } else if let Ok(float_val) = value.parse::<f64>() {
                    Ok(Value::Number(
                        serde_json::Number::from_f64(float_val)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ))
                } else {
                    Ok(Value::String(value.clone()))
                }
            }
            None => Err(ConfigError::KeyNotFound { key: key.to_string() }),
        }
    }
    
    async fn get_section(&self, section_name: &str) -> Result<ConfigSection, ConfigError> {
        let mut section = ConfigSection::new();
        let section_prefix = format!("{}.", section_name);
        
        for (key, value) in &self.env_vars {
            if key.starts_with(&section_prefix) {
                let sub_key = key.strip_prefix(&section_prefix).unwrap();
                let json_value = if let Ok(bool_val) = value.parse::<bool>() {
                    Value::Bool(bool_val)
                } else if let Ok(int_val) = value.parse::<i64>() {
                    Value::Number(serde_json::Number::from(int_val))
                } else if let Ok(float_val) = value.parse::<f64>() {
                    Value::Number(
                        serde_json::Number::from_f64(float_val)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    )
                } else {
                    Value::String(value.clone())
                };
                
                section.insert(sub_key.to_string(), json_value);
            }
        }
        
        if section.data.is_empty() {
            Err(ConfigError::KeyNotFound {
                key: section_name.to_string(),
            })
        } else {
            Ok(section)
        }
    }
    
    async fn reload(&mut self) -> Result<(), ConfigError> {
        self.load_env_vars()
    }
    
    async fn contains_key(&self, key: &str) -> Result<bool, ConfigError> {
        Ok(self.env_vars.contains_key(key))
    }
    
    async fn get_all_keys(&self) -> Result<Vec<String>, ConfigError> {
        Ok(self.env_vars.keys().cloned().collect())
    }
    
    fn name(&self) -> &str {
        "EnvironmentConfigProvider"
    }
    
    fn priority(&self) -> i32 {
        self.priority
    }
    
    fn supports_hot_reload(&self) -> bool {
        true
    }
}

#[async_trait]
impl EnvironmentConfigProviderTrait for EnvironmentConfigProviderImpl {
    fn prefix(&self) -> &str {
        &self.prefix
    }
    
    fn separator(&self) -> &str {
        &self.separator
    }
    
    async fn get_matching_env_vars(&self) -> Result<HashMap<String, String>, ConfigError> {
        Ok(self.env_vars.clone())
    }
}
