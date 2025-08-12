# GeoTargeting结构体重命名迁移指南

## 概述

为了更明确地区分不同类型的地理定向功能，我们对地理定向相关结构体进行了重命名和重构。本指南将帮助您了解变更内容并完成代码迁移。

## 变更摘要

### 结构体名变更

| 旧结构体名     | 新结构体名                   | 功能说明                             |
| -------------- | ---------------------------- | ------------------------------------ |
| `GeoTargeting` | `AdministrativeGeoTargeting` | 行政区划地理定向（国家、省份、城市） |
| -              | `CircularGeoFenceTargeting`  | 圆形地理围栏定向（已存在，无变更）   |
| -              | `PolygonGeoFenceTargeting`   | 多边形地理围栏定向（已存在，无变更） |

### 文件路径变更

| 旧文件路径                                                           | 新文件路径                                                                          |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `crates/04-core/domain/src/value_objects/targeting/geo_targeting.rs` | `crates/04-core/domain/src/value_objects/targeting/administrative_geo_targeting.rs` |

## 详细变更内容

### 1. AdministrativeGeoTargeting 结构体

**变更前：**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoTargeting {
    pub criteria_type: String, // "Geo"
    // ... 其他字段
}

impl TargetingCriteriaBase for GeoTargeting {
    fn criteria_type(&self) -> &str {
        "Geo"
    }
    // ... 其他实现
}
```

**变更后：**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdministrativeGeoTargeting {
    pub criteria_type: String, // "AdministrativeGeo"
    // ... 其他字段（功能不变）
}

impl TargetingCriteriaBase for AdministrativeGeoTargeting {
    fn criteria_type(&self) -> &str {
        "AdministrativeGeo"
    }
    // ... 其他实现（功能不变）
}
```

### 2. TargetingConfig 结构体更新

**变更前：**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetingConfig {
    pub geo_targeting: Option<GeoTargeting>,
    // ... 其他字段
}

impl TargetingConfig {
    pub fn new(
        geo_targeting: Option<GeoTargeting>,
        // ... 其他参数
    ) -> Self {
        // ... 实现
    }
}
```

**变更后：**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetingConfig {
    pub administrative_geo_targeting: Option<AdministrativeGeoTargeting>,
    pub circular_geo_fence_targeting: Option<CircularGeoFenceTargeting>,
    pub polygon_geo_fence_targeting: Option<PolygonGeoFenceTargeting>,
    // ... 其他字段
}

impl TargetingConfig {
    pub fn new(
        administrative_geo_targeting: Option<AdministrativeGeoTargeting>,
        circular_geo_fence_targeting: Option<CircularGeoFenceTargeting>,
        polygon_geo_fence_targeting: Option<PolygonGeoFenceTargeting>,
        // ... 其他参数
    ) -> Self {
        // ... 实现
    }
    
    // 新增便捷方法
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

### 3. TargetingPolicy 结构体更新

**变更前：**
```rust
impl TargetingPolicy {
    pub fn geo_targeting(&self) -> Option<&GeoTargeting> {
        self.get_criteria::<GeoTargeting>("Geo")
    }
}
```

**变更后：**
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

    // 新增便捷方法
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

## 迁移步骤

### 步骤 1：更新引用

在所有使用 `GeoTargeting` 的代码文件中：

1. 将 `GeoTargeting` 替换为 `AdministrativeGeoTargeting`
2. 如果使用 `criteria_type()` 进行匹配，将 `"Geo"` 替换为 `"AdministrativeGeo"`

### 步骤 2：更新创建代码

**变更前：**

```rust
let geo_targeting = GeoTargeting::new(
    included_locations: cities,
    mode: GeoTargetingMode::Include,
)?;

let config = TargetingConfig::new(
    geo_targeting: Some(geo_targeting),
    // ... 其他参数
)?;
```

**变更后：**

```rust
let admin_geo_targeting = AdministrativeGeoTargeting::new(
    included_locations: cities,
    mode: GeoTargetingMode::Include,
)?;

let config = TargetingConfig::new(
    administrative_geo_targeting: Some(admin_geo_targeting),
    circular_geo_fence_targeting: None,
    polygon_geo_fence_targeting: None,
    // ... 其他参数
)?;
```

### 步骤 3：更新访问代码

**变更前：**

```rust
if let Some(ref geo_targeting) = config.geo_targeting {
    let mode = &geo_targeting.mode;
}

if let Some(geo_targeting) = policy.geo_targeting() {
    let locations = &geo_targeting.included_locations;
}
```

**变更后：**

```rust
if let Some(ref admin_geo_targeting) = config.administrative_geo_targeting {
    let mode = &admin_geo_targeting.mode;
}

if let Some(admin_geo_targeting) = policy.administrative_geo_targeting() {
    let locations = &admin_geo_targeting.included_locations;
}
```

### 步骤 4：考虑多种地理定向组合

新架构支持同时使用多种地理定向类型：

```rust
let config = TargetingConfig::new(
    administrative_geo_targeting: Some(admin_targeting),     // 行政区划粗筛
    circular_geo_fence_targeting: Some(circular_targeting),  // 商圈精准定向
    polygon_geo_fence_targeting: Some(polygon_targeting),    // 复杂区域定向
    // ... 其他参数
)?;

// 检查是否有地理定向
if config.has_geo_targeting() {
    let geo_targeting_types: Vec<&str> = config.get_enabled_geo_targeting_criteria()
        .iter()
        .map(|c| c.criteria_type())
        .collect();
    println!("启用的地理定向类型: {}", geo_targeting_types.join(", "));
}
```

## 兼容性说明

### 数据库迁移

如果您的数据库中存储了序列化的定向配置，可能需要进行数据迁移：

1. **CriteriaType 更新**：将存储的 `"Geo"` 更新为 `"AdministrativeGeo"`
2. **结构体名更新**：将序列化数据中的结构体名从 `GeoTargeting` 更新为 `AdministrativeGeoTargeting`

### API 兼容性

如果您有对外的API接口，建议：

1. 保持向后兼容性，同时支持新旧字段名
2. 在API文档中标记旧字段为已弃用
3. 提供迁移时间表

## 验证迁移

完成迁移后，请进行以下验证：

1. **编译验证**：确保所有项目都能成功编译

```bash
cargo build --workspace
```

1. **单元测试验证**：运行相关的单元测试

```bash
cargo test --workspace --lib geo_targeting
```

1. **功能验证**：确保地理定向功能正常工作
   - 行政区划定向测试
   - 圆形围栏定向测试
   - 多边形围栏定向测试

## 常见问题

### Q: 为什么要进行这次重命名？

A: 原来的 `GeoTargeting` 名称过于宽泛，无法明确表示其具体功能。重命名为 `AdministrativeGeoTargeting` 后，可以清楚地表明这是基于行政区划的地理定向，与基于坐标的围栏定向区分开来。

### Q: 旧的功能会有变化吗？

A: 不会。`AdministrativeGeoTargeting` 的所有功能都与原来的 `GeoTargeting` 完全相同，只是结构体名和 criteria_type 发生了变更。

### Q: 如何选择合适的地理定向类型？

A:

- **AdministrativeGeoTargeting**：适合按城市、省份、国家进行的大范围定向
- **CircularGeoFenceTargeting**：适合商圈、POI周边的精准定向
- **PolygonGeoFenceTargeting**：适合复杂形状区域的精确定向

### Q: 可以同时使用多种地理定向吗？

A: 可以。新架构支持在同一个 `TargetingConfig` 中同时配置多种地理定向类型，系统会按照各自的规则进行匹配。

## 技术支持

如果在迁移过程中遇到问题，请：

1. 查看本迁移指南
2. 参考 `docs_TecDesign/地理定向系统设计说明.md`
3. 联系技术团队获取支持

---

**重要提醒**：请在生产环境部署前，在测试环境中充分验证迁移结果！
3. 联系技术团队获取支持

---

**重要提醒**：请在生产环境部署前，在测试环境中充分验证迁移结果！
