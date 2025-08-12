# GeoTargeting�ṹ��������Ǩ��ָ��

## ����

Ϊ�˸���ȷ�����ֲ�ͬ���͵ĵ������ܣ����ǶԵ�������ؽṹ����������������ع�����ָ�Ͻ��������˽������ݲ���ɴ���Ǩ�ơ�

## ���ժҪ

### �ṹ�������

| �ɽṹ����     | �½ṹ����                   | ����˵��                             |
| -------------- | ---------------------------- | ------------------------------------ |
| `GeoTargeting` | `AdministrativeGeoTargeting` | �������������򣨹��ҡ�ʡ�ݡ����У� |
| -              | `CircularGeoFenceTargeting`  | Բ�ε���Χ�������Ѵ��ڣ��ޱ����   |
| -              | `PolygonGeoFenceTargeting`   | ����ε���Χ�������Ѵ��ڣ��ޱ���� |

### �ļ�·�����

| ���ļ�·��                                                           | ���ļ�·��                                                                          |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `crates/04-core/domain/src/value_objects/targeting/geo_targeting.rs` | `crates/04-core/domain/src/value_objects/targeting/administrative_geo_targeting.rs` |

## ��ϸ�������

### 1. AdministrativeGeoTargeting �ṹ��

**���ǰ��**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoTargeting {
    pub criteria_type: String, // "Geo"
    // ... �����ֶ�
}

impl TargetingCriteriaBase for GeoTargeting {
    fn criteria_type(&self) -> &str {
        "Geo"
    }
    // ... ����ʵ��
}
```

**�����**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdministrativeGeoTargeting {
    pub criteria_type: String, // "AdministrativeGeo"
    // ... �����ֶΣ����ܲ��䣩
}

impl TargetingCriteriaBase for AdministrativeGeoTargeting {
    fn criteria_type(&self) -> &str {
        "AdministrativeGeo"
    }
    // ... ����ʵ�֣����ܲ��䣩
}
```

### 2. TargetingConfig �ṹ�����

**���ǰ��**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetingConfig {
    pub geo_targeting: Option<GeoTargeting>,
    // ... �����ֶ�
}

impl TargetingConfig {
    pub fn new(
        geo_targeting: Option<GeoTargeting>,
        // ... ��������
    ) -> Self {
        // ... ʵ��
    }
}
```

**�����**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetingConfig {
    pub administrative_geo_targeting: Option<AdministrativeGeoTargeting>,
    pub circular_geo_fence_targeting: Option<CircularGeoFenceTargeting>,
    pub polygon_geo_fence_targeting: Option<PolygonGeoFenceTargeting>,
    // ... �����ֶ�
}

impl TargetingConfig {
    pub fn new(
        administrative_geo_targeting: Option<AdministrativeGeoTargeting>,
        circular_geo_fence_targeting: Option<CircularGeoFenceTargeting>,
        polygon_geo_fence_targeting: Option<PolygonGeoFenceTargeting>,
        // ... ��������
    ) -> Self {
        // ... ʵ��
    }
    
    // ������ݷ���
    pub fn get_enabled_geo_targeting_criteria(&self) -> Vec<&dyn TargetingCriteriaBase> {
        let mut criteria = Vec::new();
        if let Some(ref admin_geo) = self.administrative_geo_targeting {
            criteria.push(admin_geo as &dyn TargetingCriteriaBase);
        }
        if let Some(ref circular_geo) = self.circular_geo_fence_targeting {
            criteria.push(circular_geo as &dyn TargetingCriteriaBase);
        }
        if let Some(ref polygon_geo) = self.polygon_geo_fence_targeting {
            criteria.push(polygon_geo as &dyn TargetingCriteriaBase);
        }
        criteria
    }
    
    pub fn has_geo_targeting(&self) -> bool {
        self.administrative_geo_targeting.is_some() ||
        self.circular_geo_fence_targeting.is_some() ||
        self.polygon_geo_fence_targeting.is_some()
    }
}
```

### 3. TargetingPolicy �ṹ�����

**���ǰ��**
```rust
impl TargetingPolicy {
    pub fn geo_targeting(&self) -> Option<&GeoTargeting> {
        self.get_criteria::<GeoTargeting>("Geo")
    }
}
```

**�����**
```rust
impl TargetingPolicy {
    pub fn administrative_geo_targeting(&self) -> Option<&AdministrativeGeoTargeting> {
        self.get_criteria::<AdministrativeGeoTargeting>("AdministrativeGeo")
    }
    
    pub fn circular_geo_fence_targeting(&self) -> Option<&CircularGeoFenceTargeting> {
        self.get_criteria::<CircularGeoFenceTargeting>("CircularGeoFence")
    }
    
    pub fn polygon_geo_fence_targeting(&self) -> Option<&PolygonGeoFenceTargeting> {
        self.get_criteria::<PolygonGeoFenceTargeting>("PolygonGeoFence")
    }

    // ������ݷ���
    pub fn get_geo_targeting_criteria(&self) -> Vec<&dyn TargetingCriteriaBase> {
        let mut criteria = Vec::new();
        if let Some(admin_geo) = self.administrative_geo_targeting() {
            criteria.push(admin_geo as &dyn TargetingCriteriaBase);
        }
        if let Some(circular_geo) = self.circular_geo_fence_targeting() {
            criteria.push(circular_geo as &dyn TargetingCriteriaBase);
        }
        if let Some(polygon_geo) = self.polygon_geo_fence_targeting() {
            criteria.push(polygon_geo as &dyn TargetingCriteriaBase);
        }
        criteria
    }
    
    pub fn has_geo_targeting(&self) -> bool {
        self.administrative_geo_targeting().is_some() ||
        self.circular_geo_fence_targeting().is_some() ||
        self.polygon_geo_fence_targeting().is_some()
    }
}
```

## Ǩ�Ʋ���

### ���� 1����������

������ʹ�� `GeoTargeting` �Ĵ����ļ��У�

1. �� `GeoTargeting` �滻Ϊ `AdministrativeGeoTargeting`
2. ���ʹ�� `criteria_type()` ����ƥ�䣬�� `"Geo"` �滻Ϊ `"AdministrativeGeo"`

### ���� 2�����´�������

**���ǰ��**

```rust
let geo_targeting = GeoTargeting::new(
    included_locations: cities,
    mode: GeoTargetingMode::Include,
)?;

let config = TargetingConfig::new(
    geo_targeting: Some(geo_targeting),
    // ... ��������
)?;
```

**�����**

```rust
let admin_geo_targeting = AdministrativeGeoTargeting::new(
    included_locations: cities,
    mode: GeoTargetingMode::Include,
)?;

let config = TargetingConfig::new(
    administrative_geo_targeting: Some(admin_geo_targeting),
    circular_geo_fence_targeting: None,
    polygon_geo_fence_targeting: None,
    // ... ��������
)?;
```

### ���� 3�����·��ʴ���

**���ǰ��**

```rust
if let Some(ref geo_targeting) = config.geo_targeting {
    let mode = &geo_targeting.mode;
}

if let Some(geo_targeting) = policy.geo_targeting() {
    let locations = &geo_targeting.included_locations;
}
```

**�����**

```rust
if let Some(ref admin_geo_targeting) = config.administrative_geo_targeting {
    let mode = &admin_geo_targeting.mode;
}

if let Some(admin_geo_targeting) = policy.administrative_geo_targeting() {
    let locations = &admin_geo_targeting.included_locations;
}
```

### ���� 4�����Ƕ��ֵ��������

�¼ܹ�֧��ͬʱʹ�ö��ֵ��������ͣ�

```rust
let config = TargetingConfig::new(
    administrative_geo_targeting: Some(admin_targeting),     // ����������ɸ
    circular_geo_fence_targeting: Some(circular_targeting),  // ��Ȧ��׼����
    polygon_geo_fence_targeting: Some(polygon_targeting),    // ����������
    // ... ��������
)?;

// ����Ƿ��е�����
if config.has_geo_targeting() {
    let geo_targeting_types: Vec<&str> = config.get_enabled_geo_targeting_criteria()
        .iter()
        .map(|c| c.criteria_type())
        .collect();
    println!("���õĵ���������: {}", geo_targeting_types.join(", "));
}
```

## ������˵��

### ���ݿ�Ǩ��

����������ݿ��д洢�����л��Ķ������ã�������Ҫ��������Ǩ�ƣ�

1. **CriteriaType ����**�����洢�� `"Geo"` ����Ϊ `"AdministrativeGeo"`
2. **�ṹ��������**�������л������еĽṹ������ `GeoTargeting` ����Ϊ `AdministrativeGeoTargeting`

### API ������

������ж����API�ӿڣ����飺

1. �����������ԣ�ͬʱ֧���¾��ֶ���
2. ��API�ĵ��б�Ǿ��ֶ�Ϊ������
3. �ṩǨ��ʱ���

## ��֤Ǩ��

���Ǩ�ƺ������������֤��

1. **������֤**��ȷ��������Ŀ���ܳɹ�����

```bash
cargo build --workspace
```

1. **��Ԫ������֤**��������صĵ�Ԫ����

```bash
cargo test --workspace --lib geo_targeting
```

1. **������֤**��ȷ������������������
   - ���������������
   - Բ��Χ���������
   - �����Χ���������

## ��������

### Q: ΪʲôҪ���������������

A: ԭ���� `GeoTargeting` ���ƹ��ڿ����޷���ȷ��ʾ����幦�ܡ�������Ϊ `AdministrativeGeoTargeting` �󣬿�������ر������ǻ������������ĵ���������������Χ���������ֿ�����

### Q: �ɵĹ��ܻ��б仯��

A: ���ᡣ`AdministrativeGeoTargeting` �����й��ܶ���ԭ���� `GeoTargeting` ��ȫ��ͬ��ֻ�ǽṹ������ criteria_type �����˱����

### Q: ���ѡ����ʵĵ��������ͣ�

A:

- **AdministrativeGeoTargeting**���ʺϰ����С�ʡ�ݡ����ҽ��еĴ�Χ����
- **CircularGeoFenceTargeting**���ʺ���Ȧ��POI�ܱߵľ�׼����
- **PolygonGeoFenceTargeting**���ʺϸ�����״����ľ�ȷ����

### Q: ����ͬʱʹ�ö��ֵ�������

A: ���ԡ��¼ܹ�֧����ͬһ�� `TargetingConfig` ��ͬʱ���ö��ֵ��������ͣ�ϵͳ�ᰴ�ո��ԵĹ������ƥ�䡣

## ����֧��

�����Ǩ�ƹ������������⣬�룺

1. �鿴��Ǩ��ָ��
2. �ο� `docs_TecDesign/������ϵͳ���˵��.md`
3. ��ϵ�����Ŷӻ�ȡ֧��

---

**��Ҫ����**������������������ǰ���ڲ��Ի����г����֤Ǩ�ƽ����
3. ��ϵ�����Ŷӻ�ȡ֧��

---

**��Ҫ����**������������������ǰ���ڲ��Ի����г����֤Ǩ�ƽ����
