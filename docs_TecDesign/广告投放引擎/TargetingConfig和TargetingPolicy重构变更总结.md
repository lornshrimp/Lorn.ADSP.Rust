# TargetingConfig �� TargetingPolicy �ع�����ܽ�

## ����

��������ģ�ͷֲ�����ĵ���Ҫ�󣬶� `TargetingConfig` �� `TargetingPolicy` �������Ľṹ��������ع��������ϵͳ�Ŀ���չ�Ժ�����ԡ���Ҫ�������ʹ��HashMap�ṹ��������������ǿ�ṹ��Ĺ��ܶ�λ����Ӷ�̬�Ż�������

## ��Ҫ�������

### 1. TargetingConfig �ṹ���ع�

#### 1.1 �ܹ�����

**���ǰ��**
- Ӳ������ֶ��������ֶ�
- ���޵���չ����
- ȱ����̬�Ż�����

**�����**
- ʹ��HashMap�ṹ `HashMap<String, Box<dyn TargetingCriteria>>` ����������
- ���Ӷ�̬����֧�� `HashMap<String, serde_json::Value>`
- ʵ���������������ڹ���

#### 1.2 ���������ֶ�

```rust
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetingConfig {
    // �������ñ�ʶ�͹�����Ϣ
    pub config_id: String,
    pub advertisement_id: String,
    pub source_policy_id: Option<String>,

    // HashMap�ṹ֧�ֿ���չ��
    criteria: HashMap<String, Box<dyn TargetingCriteria>>,
    dynamic_parameters: HashMap<String, serde_json::Value>,

    // �������ڹ���
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_from: String,
}

impl TargetingConfig {
    // �ṩֻ������
    pub fn criteria(&self) -> &HashMap<String, Box<dyn TargetingCriteria>> {
        &self.criteria
    }
    
    pub fn dynamic_parameters(&self) -> &HashMap<String, serde_json::Value> {
        &self.dynamic_parameters
    }
}
```

#### 1.3 �������ķ���

**����������**

- `create_from_policy()` - �� TargetingPolicy ��������ʵ��
- `create_from_scratch()` - ��ͷ��������ʵ��

**������������**

- `add_criteria()` - ��Ӷ�������
- `update_criteria()` - ���¶�������
- `remove_criteria()` - �Ƴ���������
- `get_criteria<T>()` - ��ȡָ�����͵Ķ�������
- `has_criteria()` - ����Ƿ����ָ������

**��̬��������**

- `set_dynamic_parameter()` - ���ö�̬����
- `get_dynamic_parameter<T>()` - ��ȡ��̬����

**�Ż����ܣ�**

- `apply_dynamic_optimization()` - Ӧ�ö�̬�Ż�
- `validate_config()` - ��֤������Ч��
- `clone()` - ��¡����

### 2. TargetingPolicy �ṹ���ع�

#### 2.1 ���ܶ�λ��ȷ

**���ǰ��**

- ���ܶ�λģ��
- ȱ���汾����
- û��״̬����

**�����**

- ��ȷ��λΪ�ɸ��õĶ������ģ��
- ���������İ汾�����״̬����
- ֧�ֲ��Եķ������鵵��ʹ��ͳ��

#### 2.2 ���������ֶ�

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetingPolicy {
    // ���Ա�ʶ��Ԫ����
    pub policy_id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: i32,
    pub created_by: String,

    // ״̬�ͷ������
    pub status: PolicyStatus,
    pub category: String,
    pub is_public: bool,

    // ��ǩ��ģ�����
    tags: Vec<String>,
    criteria_templates: HashMap<String, Box<dyn TargetingCriteria>>,
}

impl TargetingPolicy {
    // �ṩֻ������
    pub fn tags(&self) -> &Vec<String> {
        &self.tags
    }
    
    pub fn criteria_templates(&self) -> &HashMap<String, Box<dyn TargetingCriteria>> {
        &self.criteria_templates
    }
}
```

#### 2.3 �������ķ���

**����������**

- `create_empty()` - �����ղ���ģ��
- `create_unrestricted()` - ���������Ʋ���
- `create_config()` - ���� TargetingConfig ʵ��

**ģ�����**

- `add_criteria_template()` - �������ģ��
- `remove_criteria_template()` - �Ƴ�����ģ��
- `get_criteria_template<T>()` - ��ȡָ�����͵�����ģ��

**״̬����**

- `publish()` - ��������
- `archive()` - �鵵����
- `clone()` - ��¡����

**��ǩ����**

- `add_tag()` - ��ӱ�ǩ
- `remove_tag()` - �Ƴ���ǩ

**ʹ��ͳ�ƣ�**

- `get_usage_statistics()` - ��ȡʹ��ͳ��

### 3. ֧����������

#### 3.1 ValidationResult �ṹ��

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    
    pub fn errors(&self) -> &Vec<String> {
        &self.errors
    }
    
    pub fn warnings(&self) -> &Vec<String> {
        &self.warnings
    }
    
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
    
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}
```

#### 3.2 OptimizationContext �ṹ��

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationContext {
    pub performance_metrics: Option<PerformanceMetrics>,
    pub optimization_recommendations: Option<Vec<OptimizationRecommendation>>,
    pub additional_data: Option<HashMap<String, serde_json::Value>>,
}
```

#### 3.3 PolicyStatus ö��

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyStatus {
    Draft = 1,      // �ݸ�״̬
    Published = 2,  // �ѷ���
    Archived = 3,   // �ѹ鵵
}
```

#### 3.4 PolicyUsageStats �ṹ��

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyUsageStats {
    pub policy_id: String,
    pub total_configs: i32,
    pub active_configs: i32,
    pub last_used_at: Option<DateTime<Utc>>,
    pub average_performance: f64,
}
```

## �������

### 1. ����չ����ǿ

**HashMap�ṹ����**

- ֧�ֶ�̬����µĶ�����������
- �����޸ĺ��Ĵ��뼴����չ����
- ��������ͨ���ַ��������й�����������

**��̬����֧�֣�**

- ֧������ʱ��̬��������
- Ϊ����ѧϰ�Ż�Ԥ���ӿ�
- ֧�� A/B ���Ժ͸��Ի��Ż�

### 2. ���������

**ģ���ʵ�����룺**

- TargetingPolicy ��Ϊ�ɸ���ģ��
- TargetingConfig ��Ϊ����ʱʵ��
- ֧�ִ�ģ�崴�����ã�ͬʱ������Ի�����

**״̬�������ƣ�**

- �������������ڹ���
- ֧�ְ汾���ƺ�״̬����
- �ṩʹ��ͳ�ƺ����ܷ���

### 3. ҵ���ֵ��ǿ

**��ӪЧ��������**

- ֧�ֲ���ģ��ĸ��ú͹���
- �ṩ����ʹ��ͳ�ƺ�Ч������
- ֧�ֲ��Եķ������ͱ�ǩ����

**�����Ż�������**

- ֧�ֻ�����ʷ���ֵĶ�̬�Ż�
- �ṩ�Ż�������Զ�����
- Ϊ����ѧϰ�������Ż�Ԥ���ӿ�

## �����Դ���

### 1. ������

**������ݷ��ʷ�����**

```rust
// TargetingConfig �б����ı�ݷ���
impl TargetingConfig {
    pub fn get_enabled_geo_targeting_criteria(&self) -> Vec<&dyn TargetingCriteria> {
        // ʵ���߼�
    }
    
    pub fn has_geo_targeting(&self) -> bool {
        // ʵ���߼�
    }
}

// TargetingPolicy �б����ı�ݷ���  
impl TargetingPolicy {
    pub fn administrative_geo_targeting(&self) -> Option<&AdministrativeGeoTargeting> {
        self.get_criteria_template::<AdministrativeGeoTargeting>("AdministrativeGeo")
    }
    
    pub fn get_geo_targeting_criteria_templates(&self) -> Vec<&dyn TargetingCriteria> {
        // ʵ���߼�
    }
}
```

### 2. Ǩ��ָ��

**���д���Ǩ�ƣ�**

1. ��ֱ���ֶη��ʸ�ΪHashMap����
2. ʹ���µĴ�������������캯��
3. ������������ʽ

**��������Ǩ�ƣ�**

1. ��Ӳ��������ת��ΪHashMap�ṹ
2. ��ӱ�Ҫ��Ԫ������Ϣ
3. �������ݵ������Ժ�һ����

## Ӱ�췶Χ

### 1. ֱ��Ӱ��

**�޸ĵ��ļ���**

- `crates/04-core/domain/src/value_objects/targeting_config.rs` - ��ȫ�ع�
- `crates/04-core/domain/src/value_objects/targeting_policy.rs` - ��ȫ�ع�
- `crates/04-core/domain/src/entities/campaign.rs` - ���÷�������
- `crates/04-core/domain/src/entities/advertisement.rs` - ���÷�������

### 2. Ǳ��Ӱ��

**��Ҫ���������ģ�飺**

- ������Լ�����ʵ��
- ���Ͷ�����漯��
- ���ù���ͳ־û�
- API �ӿں����ݴ������

## ������������

### 1. ��������

1. **ʵ���������¡��** ���� `clone()` �����е����¡�߼�
2. **��ӵ�Ԫ���ԣ�** Ϊ�¹��ܱ�дȫ��ĵ�Ԫ����
3. **�����ĵ���** ���� API �ĵ���ʹ��ָ��
4. **���ܲ��ԣ�** ��֤HashMap�ṹ�����ܱ���

### 2. ��������

1. **���ɲ��ԣ�** ����Ͷ��������м��ɲ���
2. **����Ǩ�ƹ��ߣ�** �����������ݵ�Ǩ�ƹ���
3. **���ָ�꣺** ����¹��ܵļ�غͶ���ָ��
4. **�Ż��㷨��** ʵ�ֶ�̬�Ż��ľ����㷨

### 3. ���ڹ滮

1. **����ѧϰ���ɣ�** ���ɻ���ѧϰ�������Ż�
2. **���ӻ����ߣ�** �����������õĿ��ӻ�����
3. **�����Ƽ���** ����ʹ��ͳ�ƵĲ����Ƽ�
4. **A/B ���Կ�ܣ�** ���� A/B ����֧��

## �ܽ�

�˴��ع����������˶�������ϵͳ�Ŀ���չ�Ժ�����ԣ�ͨ��HashMap�ṹ��������������ȷ�� TargetingPolicy �� TargetingConfig ��ְ��ֹ���Ϊ�����Ĺ�����չ�������Ż��춨�˼�ʵ�������ع����ϵͳ���õ�֧����ҵ������Ŀ��ٱ仯�ͼ����ܹ��ĳ����ݽ���
