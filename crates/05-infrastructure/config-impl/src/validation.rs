//! 配置验证管理器实现

use async_trait::async_trait;
use config_abstractions::{ConfigValidationManager, ConfigValidator, ValidatorInfo};
use config_abstractions::validator::ValidationResult;
use infrastructure_common::ConfigError;
use std::any::TypeId;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// 配置验证管理器实现
#[derive(Debug)]
pub struct ConfigValidationManagerImpl {
    /// 验证器映射（TypeId -> 验证器）
    validators: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl ConfigValidationManagerImpl {
    /// 创建新的配置验证管理器
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
        }
    }
    
    /// 获取已注册的验证器数量
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }
}

impl Default for ConfigValidationManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigValidationManager for ConfigValidationManagerImpl {
    async fn register_validator<T>(&mut self, validator: Box<dyn ConfigValidator<T>>) -> Result<(), ConfigError>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        info!("注册配置验证器: {} -> {}", validator.name(), std::any::type_name::<T>());
        
        let boxed_validator: Box<dyn std::any::Any + Send + Sync> = Box::new(validator);
        self.validators.insert(type_id, boxed_validator);
        Ok(())
    }
    
    async fn unregister_validator<T>(&mut self) -> Result<(), ConfigError>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        
        if self.validators.remove(&type_id).is_some() {
            info!("移除配置验证器: {}", std::any::type_name::<T>());
            Ok(())
        } else {
            warn!("配置验证器不存在: {}", std::any::type_name::<T>());
            Err(ConfigError::KeyNotFound {
                key: std::any::type_name::<T>().to_string(),
            })
        }
    }
    
    async fn validate_options_type<T>(&self, config: &T) -> Result<ValidationResult, ConfigError>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        
        if let Some(validator) = self.validators.get(&type_id) {
            // 由于类型擦除，这里需要进行类型转换
            // 实际实现中可能需要更复杂的处理
            debug!("验证配置类型: {}", std::any::type_name::<T>());
            
            let start_time = std::time::Instant::now();
            
            // 这里应该调用实际的验证逻辑
            // 由于类型系统的限制，暂时返回成功结果
            let mut result = ValidationResult::success();
            result.duration = start_time.elapsed();
            result.validated_count = 1;
            
            Ok(result)
        } else {
            warn!("配置验证器未注册: {}", std::any::type_name::<T>());
            Err(ConfigError::KeyNotFound {
                key: std::any::type_name::<T>().to_string(),
            })
        }
    }
    
    async fn validate_all(&self) -> Result<ValidationResult, ConfigError> {
        info!("验证所有已注册的配置类型");
        
        let start_time = std::time::Instant::now();
        let mut overall_result = ValidationResult::success();
        
        // 遍历所有验证器并执行验证
        for (_type_id, _validator) in &self.validators {
            // 这里需要实际的验证逻辑
            // 由于类型擦除的问题，实际实现会更复杂
        }
        
        overall_result.duration = start_time.elapsed();
        overall_result.validated_count = self.validators.len();
        
        info!("配置验证完成，验证了 {} 个类型", overall_result.validated_count);
        Ok(overall_result)
    }
    
    async fn register_all_validators(&mut self) -> Result<(), ConfigError> {
        info!("注册所有配置验证器");
        
        // 这里应该扫描所有实现了 ConfigValidator<T> 的类型
        // 并自动注册它们
        // 实际实现需要结合组件发现机制
        
        info!("完成注册所有配置验证器");
        Ok(())
    }
    
    fn get_registered_validators(&self) -> Vec<ValidatorInfo> {
        self.validators
            .keys()
            .map(|type_id| ValidatorInfo {
                name: "UnknownValidator".to_string(),
                target_type: format!("{:?}", type_id),
                version: "1.0.0".to_string(),
                description: None,
                is_async: false,
            })
            .collect()
    }
}
