//! 高级配置验证器实现

use async_trait::async_trait;
use config_abstractions::manager::{
    ValidationError, ValidationErrorType, ValidationResult, ValidationWarning,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tracing::{debug, error, info};

/// 高级配置验证器
///
/// 提供复合验证规则、依赖检查和自定义验证逻辑
pub struct AdvancedConfigValidator {
    /// 验证规则映射
    rules: HashMap<String, Vec<Box<dyn ValidationRule>>>,
    /// 依赖关系图
    dependencies: HashMap<String, Vec<String>>,
    /// 是否启用严格模式
    strict_mode: bool,
    /// 自定义验证器
    custom_validators: HashMap<String, Box<dyn CustomValidator>>,
}

impl AdvancedConfigValidator {
    /// 创建新的高级配置验证器
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            dependencies: HashMap::new(),
            strict_mode: false,
            custom_validators: HashMap::new(),
        }
    }

    /// 启用严格模式
    pub fn with_strict_mode(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// 添加验证规则
    pub fn add_rule<R: ValidationRule + 'static>(&mut self, path: &str, rule: R) {
        self.rules
            .entry(path.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(rule));
    }

    /// 添加配置依赖关系
    pub fn add_dependency(&mut self, config_path: &str, depends_on: &str) {
        self.dependencies
            .entry(config_path.to_string())
            .or_insert_with(Vec::new)
            .push(depends_on.to_string());
    }

    /// 添加自定义验证器
    pub fn add_custom_validator<V: CustomValidator + 'static>(&mut self, name: &str, validator: V) {
        self.custom_validators
            .insert(name.to_string(), Box::new(validator));
    }

    /// 验证单个配置项
    pub async fn validate_config_item(&self, path: &str, value: &Value) -> ValidationResult {
        let mut result = ValidationResult::success();

        // 检查是否有验证规则
        if let Some(rules) = self.rules.get(path) {
            for rule in rules {
                let rule_result = rule.validate(value).await;
                if !rule_result.is_valid {
                    result.is_valid = false;
                    result.errors.extend(rule_result.errors);
                }
                result.warnings.extend(rule_result.warnings);
            }
        }

        // 检查依赖关系
        if let Err(dep_errors) = self.check_dependencies(path).await {
            result.is_valid = false;
            result.errors.extend(dep_errors);
        }

        result
    }

    /// 验证所有配置
    pub async fn validate_all_configs(&self, configs: &HashMap<String, Value>) -> ValidationResult {
        let mut overall_result = ValidationResult::success();

        info!("开始高级配置验证，配置项数量: {}", configs.len());

        for (path, value) in configs {
            let item_result = self.validate_config_item(path, value).await;

            if !item_result.is_valid {
                overall_result.is_valid = false;
                overall_result.errors.extend(item_result.errors);
            }
            overall_result.warnings.extend(item_result.warnings);
        }

        // 检查全局依赖关系
        if let Err(global_errors) = self.check_global_dependencies(configs).await {
            overall_result.is_valid = false;
            overall_result.errors.extend(global_errors);
        }

        // 运行自定义验证器
        for (name, validator) in &self.custom_validators {
            debug!("运行自定义验证器: {}", name);
            if let Err(custom_errors) = validator.validate(configs).await {
                overall_result.is_valid = false;
                overall_result.errors.extend(custom_errors);
            }
        }

        overall_result.validated_at = chrono::Utc::now();

        if overall_result.is_valid {
            info!("高级配置验证通过");
        } else {
            error!("高级配置验证失败，错误数: {}", overall_result.errors.len());
        }

        overall_result
    }

    /// 检查单个配置的依赖关系
    async fn check_dependencies(&self, config_path: &str) -> Result<(), Vec<ValidationError>> {
        if let Some(deps) = self.dependencies.get(config_path) {
            let mut errors = Vec::new();

            for dep in deps {
                // 这里应该检查依赖的配置是否存在和有效
                // 暂时跳过实际检查，在实际实现中需要完善
                debug!("检查依赖关系: {} -> {}", config_path, dep);
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        } else {
            Ok(())
        }
    }

    /// 检查全局依赖关系
    async fn check_global_dependencies(
        &self,
        _configs: &HashMap<String, Value>,
    ) -> Result<(), Vec<ValidationError>> {
        // 实现全局依赖关系检查
        // 检查循环依赖、缺失依赖等

        if let Some(cycle) = self.detect_dependency_cycles() {
            let error = ValidationError::new(
                "dependency",
                format!("检测到配置依赖循环: {}", cycle),
                ValidationErrorType::Custom,
            );
            return Err(vec![error]);
        }

        Ok(())
    }

    /// 检测依赖循环
    fn detect_dependency_cycles(&self) -> Option<String> {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for config_path in self.dependencies.keys() {
            if self.has_cycle_util(config_path, &mut visited, &mut recursion_stack) {
                return Some(format!("涉及配置: {}", config_path));
            }
        }

        None
    }

    /// 递归检查依赖循环
    fn has_cycle_util(
        &self,
        config_path: &str,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(config_path.to_string());
        recursion_stack.insert(config_path.to_string());

        if let Some(deps) = self.dependencies.get(config_path) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycle_util(dep, visited, recursion_stack) {
                        return true;
                    }
                } else if recursion_stack.contains(dep) {
                    return true;
                }
            }
        }

        recursion_stack.remove(config_path);
        false
    }
}

impl Default for AdvancedConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 验证规则 trait
#[async_trait]
pub trait ValidationRule: Send + Sync {
    /// 验证配置值
    async fn validate(&self, value: &Value) -> ValidationResult;

    /// 获取规则名称
    fn name(&self) -> &str;

    /// 获取规则描述
    fn description(&self) -> &str;
}

/// 自定义验证器 trait
#[async_trait]
pub trait CustomValidator: Send + Sync {
    /// 验证整个配置集合
    async fn validate(&self, configs: &HashMap<String, Value>) -> Result<(), Vec<ValidationError>>;

    /// 获取验证器名称
    fn name(&self) -> &str;
}

/// 必需值验证规则
pub struct RequiredRule {
    name: String,
}

impl RequiredRule {
    pub fn new() -> Self {
        Self {
            name: "RequiredRule".to_string(),
        }
    }
}

impl Default for RequiredRule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationRule for RequiredRule {
    async fn validate(&self, value: &Value) -> ValidationResult {
        let mut result = ValidationResult::success();

        if value.is_null() {
            result.is_valid = false;
            result
                .errors
                .push(ValidationError::required_field_missing("value"));
        }

        result
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "验证配置值不能为空"
    }
}

/// 范围验证规则
pub struct RangeRule {
    name: String,
    min: Option<f64>,
    max: Option<f64>,
}

impl RangeRule {
    pub fn new(min: Option<f64>, max: Option<f64>) -> Self {
        Self {
            name: "RangeRule".to_string(),
            min,
            max,
        }
    }

    pub fn min_value(min: f64) -> Self {
        Self::new(Some(min), None)
    }

    pub fn max_value(max: f64) -> Self {
        Self::new(None, Some(max))
    }

    pub fn between(min: f64, max: f64) -> Self {
        Self::new(Some(min), Some(max))
    }
}

#[async_trait]
impl ValidationRule for RangeRule {
    async fn validate(&self, value: &Value) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(num) = value.as_f64() {
            if let Some(min) = self.min {
                if num < min {
                    result.is_valid = false;
                    result.errors.push(ValidationError::invalid_field_value(
                        "value",
                        num.to_string(),
                        format!("值必须大于等于 {}", min),
                    ));
                }
            }

            if let Some(max) = self.max {
                if num > max {
                    result.is_valid = false;
                    result.errors.push(ValidationError::invalid_field_value(
                        "value",
                        num.to_string(),
                        format!("值必须小于等于 {}", max),
                    ));
                }
            }
        } else {
            result.warnings.push(ValidationWarning::new(
                "value",
                "值不是数字类型，跳过范围验证",
                Some("请确保值为数字类型".to_string()),
            ));
        }

        result
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "验证数值在指定范围内"
    }
}

/// 字符串长度验证规则
pub struct StringLengthRule {
    name: String,
    min_length: Option<usize>,
    max_length: Option<usize>,
}

impl StringLengthRule {
    pub fn new(min_length: Option<usize>, max_length: Option<usize>) -> Self {
        Self {
            name: "StringLengthRule".to_string(),
            min_length,
            max_length,
        }
    }

    pub fn min_length(min: usize) -> Self {
        Self::new(Some(min), None)
    }

    pub fn max_length(max: usize) -> Self {
        Self::new(None, Some(max))
    }

    pub fn length_range(min: usize, max: usize) -> Self {
        Self::new(Some(min), Some(max))
    }
}

#[async_trait]
impl ValidationRule for StringLengthRule {
    async fn validate(&self, value: &Value) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(s) = value.as_str() {
            let length = s.len();

            if let Some(min) = self.min_length {
                if length < min {
                    result.is_valid = false;
                    result.errors.push(ValidationError::invalid_field_value(
                        "value",
                        s.to_string(),
                        format!("字符串长度必须大于等于 {}", min),
                    ));
                }
            }

            if let Some(max) = self.max_length {
                if length > max {
                    result.is_valid = false;
                    result.errors.push(ValidationError::invalid_field_value(
                        "value",
                        s.to_string(),
                        format!("字符串长度必须小于等于 {}", max),
                    ));
                }
            }
        } else {
            result.warnings.push(ValidationWarning::new(
                "value",
                "值不是字符串类型，跳过长度验证",
                Some("请确保值为字符串类型".to_string()),
            ));
        }

        result
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "验证字符串长度在指定范围内"
    }
}

/// 正则表达式验证规则
pub struct RegexRule {
    name: String,
    pattern: regex::Regex,
    description: String,
}

impl RegexRule {
    pub fn new(pattern: &str, description: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            name: "RegexRule".to_string(),
            pattern: regex::Regex::new(pattern)?,
            description: description.to_string(),
        })
    }

    pub fn email() -> Result<Self, regex::Error> {
        Self::new(
            r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$",
            "验证电子邮件地址格式",
        )
    }

    pub fn url() -> Result<Self, regex::Error> {
        Self::new(r"^https?://[^\s/$.?#].[^\s]*$", "验证URL格式")
    }
}

#[async_trait]
impl ValidationRule for RegexRule {
    async fn validate(&self, value: &Value) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(s) = value.as_str() {
            if !self.pattern.is_match(s) {
                result.is_valid = false;
                result.errors.push(ValidationError::format_error(
                    "value",
                    self.pattern.as_str(),
                ));
            }
        } else {
            result.warnings.push(ValidationWarning::new(
                "value",
                "值不是字符串类型，跳过正则表达式验证",
                Some("请确保值为字符串类型".to_string()),
            ));
        }

        result
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// 业务逻辑验证器示例
pub struct AdEngineConfigValidator {
    name: String,
}

impl AdEngineConfigValidator {
    pub fn new() -> Self {
        Self {
            name: "AdEngineConfigValidator".to_string(),
        }
    }
}

impl Default for AdEngineConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CustomValidator for AdEngineConfigValidator {
    async fn validate(&self, configs: &HashMap<String, Value>) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // 验证广告引擎特定的业务逻辑
        // 例如：检查广告投放引擎配置的一致性

        if let Some(engine_config) = configs.get("ad_engine") {
            if let Some(obj) = engine_config.as_object() {
                // 检查必需的引擎配置项
                let required_fields = ["max_concurrent_requests", "timeout_ms", "retry_attempts"];
                for field in &required_fields {
                    if !obj.contains_key(*field) {
                        errors.push(ValidationError::required_field_missing(*field));
                    }
                }

                // 检查配置值的合理性
                if let Some(max_requests) =
                    obj.get("max_concurrent_requests").and_then(|v| v.as_u64())
                {
                    if max_requests > 10000 {
                        errors.push(ValidationError::invalid_field_value(
                            "max_concurrent_requests",
                            max_requests.to_string(),
                            "并发请求数不应超过10000".to_string(),
                        ));
                    }
                }

                if let Some(timeout) = obj.get("timeout_ms").and_then(|v| v.as_u64()) {
                    if timeout < 100 || timeout > 30000 {
                        errors.push(ValidationError::value_out_of_range(
                            "timeout_ms",
                            timeout.to_string(),
                            "100-30000".to_string(),
                        ));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_required_rule() {
        let rule = RequiredRule::new();

        // 测试空值
        let result = rule.validate(&Value::Null).await;
        assert!(!result.is_valid);

        // 测试非空值
        let result = rule.validate(&json!("test")).await;
        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn test_range_rule() {
        let rule = RangeRule::between(1.0, 100.0);

        // 测试在范围内的值
        let result = rule.validate(&json!(50.0)).await;
        assert!(result.is_valid);

        // 测试超出范围的值
        let result = rule.validate(&json!(150.0)).await;
        assert!(!result.is_valid);

        // 测试非数字值
        let result = rule.validate(&json!("not a number")).await;
        assert!(result.is_valid); // 应该有警告但验证通过
        assert!(!result.warnings.is_empty());
    }

    #[tokio::test]
    async fn test_string_length_rule() {
        let rule = StringLengthRule::length_range(3, 10);

        // 测试长度在范围内的字符串
        let result = rule.validate(&json!("hello")).await;
        assert!(result.is_valid);

        // 测试长度超出范围的字符串
        let result = rule.validate(&json!("hi")).await;
        assert!(!result.is_valid);

        let result = rule.validate(&json!("this is too long")).await;
        assert!(!result.is_valid);
    }

    #[tokio::test]
    async fn test_regex_rule() {
        let rule = RegexRule::email().unwrap();

        // 测试有效的电子邮件
        let result = rule.validate(&json!("test@example.com")).await;
        assert!(result.is_valid);

        // 测试无效的电子邮件
        let result = rule.validate(&json!("invalid-email")).await;
        assert!(!result.is_valid);
    }

    #[tokio::test]
    async fn test_advanced_validator() {
        let mut validator = AdvancedConfigValidator::new();
        validator.add_rule("test.required", RequiredRule::new());
        validator.add_rule("test.number", RangeRule::between(1.0, 100.0));

        let configs = hashmap! {
            "test.required".to_string() => json!("value"),
            "test.number".to_string() => json!(50.0),
        };

        let result = validator.validate_all_configs(&configs).await;
        assert!(result.is_valid);
    }
}

// 辅助宏用于测试
#[cfg(test)]
macro_rules! hashmap {
    ($($key:expr => $value:expr),* $(,)?) => {
        {
            let mut map = HashMap::new();
            $(map.insert($key, $value);)*
            map
        }
    };
}

#[cfg(test)]
use hashmap;
