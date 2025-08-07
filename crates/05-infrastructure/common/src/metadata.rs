//! 元数据定义
//!
//! 提供组件和类型的元数据信息

use std::any::TypeId;
use std::collections::HashMap;

/// 类型信息
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeInfo {
    /// 类型名称
    pub name: String,
    /// 类型ID
    pub id: TypeId,
    /// 模块路径
    pub module_path: String,
}

impl TypeInfo {
    /// 创建新的类型信息
    pub fn new(type_id: TypeId, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: type_id,
            module_path: std::module_path!().to_string(),
        }
    }

    /// 从类型获取类型信息
    pub fn of<T: 'static>() -> Self {
        Self {
            name: std::any::type_name::<T>()
                .split("::")
                .last()
                .unwrap_or("Unknown")
                .to_string(),
            id: TypeId::of::<T>(),
            module_path: std::any::type_name::<T>().to_string(),
        }
    }

    /// 从类型名称创建类型信息（用于配置）
    pub fn from_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
            id: TypeId::of::<()>(), // 占位符，实际应该由运行时解析
            module_path: name.to_string(),
        }
    }

    /// 获取简短的类型名称（不包含模块路径）
    pub fn short_name(&self) -> &str {
        self.name.split("::").last().unwrap_or(&self.name)
    }
}

/// 组件元数据
#[derive(Debug, Clone)]
pub struct ComponentMetadata {
    /// 类型信息
    pub type_info: TypeInfo,
    /// 组件名称
    pub name: String,
    /// 组件描述
    pub description: Option<String>,
    /// 组件版本
    pub version: Option<String>,
    /// 组件作者
    pub author: Option<String>,
    /// 组件标签
    pub tags: Vec<String>,
    /// 自定义属性
    pub properties: HashMap<String, String>,
}

impl ComponentMetadata {
    /// 创建新的组件元数据
    pub fn new(type_info: TypeInfo, name: impl Into<String>) -> Self {
        Self {
            type_info,
            name: name.into(),
            description: None,
            version: None,
            author: None,
            tags: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置版本
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// 设置作者
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// 添加属性
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

/// 配置元数据
#[derive(Debug, Clone)]
pub struct ConfigurationMetadata {
    /// 配置路径
    pub path: String,
    /// 配置描述
    pub description: Option<String>,
    /// 是否必需
    pub required: bool,
    /// 默认值
    pub default_value: Option<serde_json::Value>,
    /// 验证规则
    pub validation_rules: Vec<String>,
    /// 示例值
    pub examples: Vec<serde_json::Value>,
}

impl ConfigurationMetadata {
    /// 创建新的配置元数据
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            description: None,
            required: false,
            default_value: None,
            validation_rules: Vec::new(),
            examples: Vec::new(),
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置为必需
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// 设置默认值
    pub fn with_default(mut self, value: serde_json::Value) -> Self {
        self.default_value = Some(value);
        self
    }

    /// 添加验证规则
    pub fn with_validation_rule(mut self, rule: impl Into<String>) -> Self {
        self.validation_rules.push(rule.into());
        self
    }

    /// 添加示例
    pub fn with_example(mut self, example: serde_json::Value) -> Self {
        self.examples.push(example);
        self
    }
}
