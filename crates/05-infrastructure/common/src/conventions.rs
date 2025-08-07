//! 约定规范定义
//!
//! 提供组件发现和配置的约定规范

use crate::lifecycle::Lifetime;
use crate::metadata::TypeInfo;
use std::any::TypeId;
use std::collections::HashMap;

/// 约定规则
#[derive(Debug, Clone)]
pub struct ConventionRule {
    /// 名称模式
    pub pattern: String,
    /// 默认生命周期
    pub lifetime: Lifetime,
    /// 必需的 trait（可选）
    pub required_trait: Option<TypeId>,
    /// 配置路径模板
    pub config_path_template: String,
    /// 优先级
    pub priority: i32,
}

impl ConventionRule {
    /// 创建新的约定规则
    pub fn new(
        pattern: impl Into<String>,
        lifetime: Lifetime,
        config_path_template: impl Into<String>,
    ) -> Self {
        Self {
            pattern: pattern.into(),
            lifetime,
            required_trait: None,
            config_path_template: config_path_template.into(),
            priority: 0,
        }
    }

    /// 设置必需的 trait
    pub fn with_required_trait(mut self, trait_id: TypeId) -> Self {
        self.required_trait = Some(trait_id);
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// 检查类型是否匹配此规则
    pub fn matches(&self, type_info: &TypeInfo) -> bool {
        self.pattern_matches(&type_info.name)
    }

    /// 检查模式是否匹配
    fn pattern_matches(&self, name: &str) -> bool {
        if self.pattern.contains('*') {
            // 简单的通配符匹配
            let pattern_parts: Vec<&str> = self.pattern.split('*').collect();

            if pattern_parts.len() == 2 {
                let prefix = pattern_parts[0];
                let suffix = pattern_parts[1];

                name.starts_with(prefix) && name.ends_with(suffix)
            } else {
                false
            }
        } else {
            name == self.pattern
        }
    }
}

/// 组件约定规范
#[derive(Debug)]
pub struct ComponentConventions {
    rules: Vec<ConventionRule>,
}

impl ComponentConventions {
    /// 创建新的组件约定规范
    pub fn new() -> Self {
        let mut conventions = Self { rules: Vec::new() };
        conventions.register_default_conventions();
        conventions
    }

    /// 注册默认约定
    fn register_default_conventions(&mut self) {
        // 策略组件约定
        self.add_convention(
            ConventionRule::new(
                "*Strategy",
                Lifetime::Transient,
                "strategies.{component_name}",
            )
            .with_priority(100),
        );

        // 服务组件约定
        self.add_convention(
            ConventionRule::new("*Service", Lifetime::Singleton, "services.{component_name}")
                .with_priority(90),
        );

        // 管理器组件约定
        self.add_convention(
            ConventionRule::new("*Manager", Lifetime::Singleton, "managers.{component_name}")
                .with_priority(90),
        );

        // 提供者组件约定
        self.add_convention(
            ConventionRule::new("*Provider", Lifetime::Scoped, "providers.{component_name}")
                .with_priority(80),
        );

        // 匹配器组件约定
        self.add_convention(
            ConventionRule::new("*Matcher", Lifetime::Transient, "matchers.{component_name}")
                .with_priority(70),
        );

        // 计算器组件约定
        self.add_convention(
            ConventionRule::new(
                "*Calculator",
                Lifetime::Transient,
                "calculators.{component_name}",
            )
            .with_priority(70),
        );
    }

    /// 添加约定规则
    pub fn add_convention(&mut self, rule: ConventionRule) {
        self.rules.push(rule);
        // 按优先级排序
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 获取所有约定规则
    pub fn get_convention_rules(&self) -> &[ConventionRule] {
        &self.rules
    }

    /// 根据类型查找匹配的规则
    pub fn find_rule_by_type(&self, type_info: &TypeInfo) -> Option<&ConventionRule> {
        self.rules.iter().find(|rule| rule.matches(type_info))
    }

    /// 检查类型是否为组件
    pub fn is_component(&self, type_info: &TypeInfo) -> bool {
        self.find_rule_by_type(type_info).is_some()
    }
}

impl Default for ComponentConventions {
    fn default() -> Self {
        Self::new()
    }
}

/// 命名约定规范
#[derive(Debug)]
pub struct NamingConventions;

impl NamingConventions {
    /// 从类型信息提取组件名称
    pub fn extract_component_name(type_info: &TypeInfo) -> String {
        type_info.name.clone()
    }

    /// 获取配置路径
    pub fn get_configuration_path(type_info: &TypeInfo) -> String {
        // 将驼峰命名转换为小写下划线形式
        let name = Self::to_snake_case(&type_info.name);

        // 根据后缀确定配置节
        if type_info.name.ends_with("Strategy") {
            format!("strategies.{}", name.trim_end_matches("_strategy"))
        } else if type_info.name.ends_with("Service") {
            format!("services.{}", name.trim_end_matches("_service"))
        } else if type_info.name.ends_with("Manager") {
            format!("managers.{}", name.trim_end_matches("_manager"))
        } else if type_info.name.ends_with("Provider") {
            format!("providers.{}", name.trim_end_matches("_provider"))
        } else if type_info.name.ends_with("Matcher") {
            format!("matchers.{}", name.trim_end_matches("_matcher"))
        } else if type_info.name.ends_with("Calculator") {
            format!("calculators.{}", name.trim_end_matches("_calculator"))
        } else {
            format!("components.{}", name)
        }
    }

    /// 将驼峰命名转换为蛇形命名
    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch.is_uppercase() && !result.is_empty() {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch.is_lowercase() {
                        result.push('_');
                    }
                }
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        }

        result
    }

    /// 获取服务 trait 类型
    pub fn get_service_trait(type_info: &TypeInfo) -> Option<TypeInfo> {
        // 简化实现，实际情况可能需要更复杂的逻辑
        None
    }
}

/// 配置约定规范
#[derive(Debug)]
pub struct ConfigurationConventions;

impl ConfigurationConventions {
    /// 获取配置节名称
    pub fn get_config_section_name(type_info: &TypeInfo) -> String {
        NamingConventions::get_configuration_path(type_info)
    }

    /// 获取选项类型
    pub fn get_options_type(type_info: &TypeInfo) -> Option<TypeInfo> {
        // 按约定，配置类型名称应该是 {ComponentName}Config
        let config_type_name = format!("{}Config", type_info.name);
        Some(TypeInfo {
            name: config_type_name,
            id: type_info.id, // 这里实际上应该是不同的 TypeId
            module_path: type_info.module_path.clone(),
        })
    }

    /// 验证配置路径
    pub fn validate_config_path(path: &str) -> bool {
        // 基本的配置路径验证
        !path.is_empty() && path.contains('.') && !path.starts_with('.') && !path.ends_with('.')
    }
}
