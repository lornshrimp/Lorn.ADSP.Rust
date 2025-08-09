## 10. Cargo工作空间项目架构映射 (Rust实现)

### 10.1 领域模型层项目映射

#### 10.1.1 核心实体项目分布

| 实体名称       | 所属crate               | crate类型    | 具体文件路径                         | 实现要点                        |
| -------------- | ----------------------- | ------------ | ------------------------------------ | ------------------------------- |
| Advertisement  | `crates/04-core/domain` | Rust Library | `/src/entities/advertisement.rs`     | 结构体封装、trait实现、生命周期 |
| Campaign       | `crates/04-core/domain` | Rust Library | `/src/entities/campaign.rs`          | 聚合根设计、业务规则验证        |
| Advertiser     | `crates/04-core/domain` | Rust Library | `/src/entities/advertiser.rs`        | 实体标识、状态机管理            |
| MediaResource  | `crates/04-core/domain` | Rust Library | `/src/entities/media_resource.rs`    | 媒体属性建模、配置管理          |
| DeliveryRecord | `crates/04-core/domain` | Rust Library | `/src/aggregates/delivery_record.rs` | 聚合设计、事件发布              |

```rust
// 示例：Advertisement实体实现指导
// 文件路径: crates/04-core/domain/src/entities/advertisement.rs

use crate::value_objects::TargetingPolicy;
use crate::enums::AdStatus;
use crate::events::AdCreatedEvent;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advertisement {
    // 基础属性
    id: Option<i32>,
    title: String,
    status: AdStatus,
    targeting_policy: TargetingPolicy,
    advertiser_id: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Advertisement {
    // 构造函数
    pub fn new(title: String, advertiser_id: i32, targeting_policy: TargetingPolicy) -> Self {
        Self {
            id: None,
            title,
            status: AdStatus::Draft,
            targeting_policy,
            advertiser_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // 领域行为
    pub fn submit_for_review(&mut self) -> Result<(), String> {
        // 业务逻辑验证
        if self.title.is_empty() {
            return Err("广告标题不能为空".to_string());
        }
        
        // 状态变更
        self.status = AdStatus::PendingReview;
        self.updated_at = Utc::now();
        
        // 领域事件发布（这里可以返回事件）
        Ok(())
    }
    
    // 工厂方法
    pub fn create(title: String, advertiser_id: i32) -> Result<Self, String> {
        if title.is_empty() {
            return Err("标题不能为空".to_string());
        }
        
        Ok(Self::new(
            title,
            advertiser_id,
            TargetingPolicy::default(),
        ))
    }
    
    // Getter方法
    pub fn id(&self) -> Option<i32> { self.id }
    pub fn title(&self) -> &str { &self.title }
    pub fn status(&self) -> &AdStatus { &self.status }
    pub fn targeting_policy(&self) -> &TargetingPolicy { &self.targeting_policy }
    pub fn advertiser_id(&self) -> i32 { self.advertiser_id }
}

// 实现聚合根trait
impl crate::aggregates::AggregateRoot for Advertisement {
    type Id = i32;
    
    fn id(&self) -> Option<Self::Id> {
        self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
}
```

#### 10.1.2 值对象项目分布

| 值对象名称      | 所属crate               | crate类型    | 具体文件路径                             | 实现要点                    |
| --------------- | ----------------------- | ------------ | ---------------------------------------- | --------------------------- |
| TargetingPolicy | `crates/04-core/domain` | Rust Library | `/src/value_objects/targeting_policy.rs` | 不可变性、相等性比较、Clone |
| DeliveryPolicy  | `crates/04-core/domain` | Rust Library | `/src/value_objects/delivery_policy.rs`  | 结构化数据、验证逻辑        |
| AuditInfo       | `crates/04-core/domain` | Rust Library | `/src/value_objects/audit_info.rs`       | 审核状态封装、状态机        |
| GeoLocation     | `crates/04-core/domain` | Rust Library | `/src/value_objects/geo_location.rs`     | 地理坐标、距离计算          |
| BudgetInfo      | `crates/04-core/domain` | Rust Library | `/src/value_objects/budget_info.rs`      | 预算计算、约束验证          |

```rust
// 示例：TargetingPolicy值对象实现指导
// 文件路径: crates/04-core/domain/src/value_objects/targeting_policy.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetingPolicy {
    geo_targeting: GeoTargeting,
    demographic_targeting: DemographicTargeting,
    device_targeting: DeviceTargeting,
    time_targeting: TimeTargeting,
}

impl TargetingPolicy {
    pub fn new(
        geo_targeting: GeoTargeting,
        demographic_targeting: DemographicTargeting,
        device_targeting: DeviceTargeting,
        time_targeting: TimeTargeting,
    ) -> Result<Self, String> {
        // 验证逻辑
        let policy = Self {
            geo_targeting,
            demographic_targeting,
            device_targeting,
            time_targeting,
        };
        
        policy.validate()?;
        Ok(policy)
    }
    
    pub fn validate(&self) -> Result<(), String> {
        // 组合验证逻辑
        if !self.geo_targeting.is_valid() {
            return Err("地理定向配置无效".to_string());
        }
        
        if !self.demographic_targeting.is_valid() {
            return Err("人群定向配置无效".to_string());
        }
        
        Ok(())
    }
    
    // Getter方法 (不可变访问)
    pub fn geo_targeting(&self) -> &GeoTargeting { &self.geo_targeting }
    pub fn demographic_targeting(&self) -> &DemographicTargeting { &self.demographic_targeting }
    pub fn device_targeting(&self) -> &DeviceTargeting { &self.device_targeting }
    pub fn time_targeting(&self) -> &TimeTargeting { &self.time_targeting }
}

impl Default for TargetingPolicy {
    fn default() -> Self {
        Self {
            geo_targeting: GeoTargeting::default(),
            demographic_targeting: DemographicTargeting::default(),
            device_targeting: DeviceTargeting::default(),
            time_targeting: TimeTargeting::default(),
        }
    }
}

// 相关子类型定义
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GeoTargeting {
    // 地理定向配置
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DemographicTargeting {
    // 人群定向配置
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeviceTargeting {
    // 设备定向配置
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TimeTargeting {
    // 时间定向配置
}
```

#### 10.1.3 领域事件项目分布

| 事件名称             | 所属crate               | crate类型    | 具体文件路径                            | 实现要点                 |
| -------------------- | ----------------------- | ------------ | --------------------------------------- | ------------------------ |
| AdCreatedEvent       | `crates/04-core/domain` | Rust Library | `/src/events/ad_created_event.rs`       | 事件数据、时间戳、序列化 |
| AdApprovedEvent      | `crates/04-core/domain` | Rust Library | `/src/events/ad_approved_event.rs`      | 审核结果、操作人         |
| BudgetExhaustedEvent | `crates/04-core/domain` | Rust Library | `/src/events/budget_exhausted_event.rs` | 预算信息、告警级别       |
| DeliverySuccessEvent | `crates/04-core/domain` | Rust Library | `/src/events/delivery_success_event.rs` | 投放结果、统计数据       |

### 10.2 数据传输层项目映射

#### 10.2.1 API传输对象分布

| DTO类型            | 所属crate                        | crate类型    | 具体文件路径                                 | 实现要点                   |
| ------------------ | -------------------------------- | ------------ | -------------------------------------------- | -------------------------- |
| AdRequestDTO       | `crates/01-presentation/web-api` | Binary Crate | `/src/dtos/requests/ad_request_dto.rs`       | 数据验证、格式转换、序列化 |
| AdResponseDTO      | `crates/01-presentation/web-api` | Binary Crate | `/src/dtos/responses/ad_response_dto.rs`     | 序列化优化、字段筛选       |
| BidRequestDTO      | `crates/02-services/bidding`     | Binary Crate | `/src/dtos/requests/bid_request_dto.rs`      | OpenRTB兼容性、协议映射    |
| CampaignCommandDTO | `crates/02-services/campaign`    | Binary Crate | `/src/dtos/commands/campaign_command_dto.rs` | 命令验证、业务规则         |
| TargetingQueryDTO  | `crates/02-services/targeting`   | Binary Crate | `/src/dtos/queries/targeting_query_dto.rs`   | 查询优化、索引友好         |

```rust
// 示例：AdRequestDTO实现指导
// 文件路径: crates/01-presentation/web-api/src/dtos/requests/ad_request_dto.rs

use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use crate::dtos::common::{DeviceInfoDTO, GeoLocationDTO, UserProfileDTO};
use crate::domain::requests::AdRequest;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AdRequestDTO {
    #[validate(length(min = 1, max = 50))]
    pub placement_id: String,
    
    #[validate]
    pub device: DeviceInfoDTO,
    
    #[validate]
    pub geo_location: GeoLocationDTO,
    
    pub user_profile: Option<UserProfileDTO>,
    
    #[validate(range(min = 1, max = 100))]
    pub max_ads: Option<u32>,
    
    pub request_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl AdRequestDTO {
    // 验证方法
    pub fn validate_business_rules(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // 业务验证逻辑
        if self.placement_id.is_empty() {
            errors.push("广告位ID不能为空".to_string());
        }
        
        if self.device.is_mobile() && self.user_profile.is_none() {
            errors.push("移动设备必须提供用户画像".to_string());
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    // 转换方法
    pub fn to_ad_request(self) -> Result<AdRequest, String> {
        // 验证DTO
        self.validate().map_err(|e| format!("验证失败: {:?}", e))?;
        self.validate_business_rules().map_err(|e| e.join(", "))?;
        
        // DTO到领域对象转换
        Ok(AdRequest::new(
            self.placement_id,
            self.device.into(),
            self.geo_location.into(),
            self.user_profile.map(|p| p.into()),
            self.max_ads.unwrap_or(10),
            self.request_time.unwrap_or_else(chrono::Utc::now),
        )?)
    }
}

impl Default for AdRequestDTO {
    fn default() -> Self {
        Self {
            placement_id: String::new(),
            device: DeviceInfoDTO::default(),
            geo_location: GeoLocationDTO::default(),
            user_profile: None,
            max_ads: Some(10),
            request_time: Some(chrono::Utc::now()),
        }
    }
}

// 用于Axum请求处理的FromRequest实现
#[async_trait::async_trait]
impl<S> axum::extract::FromRequest<S> for AdRequestDTO
where
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum::Json(dto) = axum::Json::<AdRequestDTO>::from_request(req, state)
            .await
            .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
        
        dto.validate()
            .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
            
        Ok(dto)
    }
}
```

#### 10.2.2 服务间通信对象分布

| 通信对象             | 所属crate               | crate类型    | 具体文件路径                             | 实现要点                      |
| -------------------- | ----------------------- | ------------ | ---------------------------------------- | ----------------------------- |
| BiddingMessage       | `crates/04-core/shared` | Rust Library | `/src/messages/bidding_message.rs`       | Protocol Buffers序列化、async |
| TargetingMessage     | `crates/04-core/shared` | Rust Library | `/src/messages/targeting_message.rs`     | 轻量级数据结构、零拷贝        |
| DeliveryNotification | `crates/04-core/shared` | Rust Library | `/src/messages/delivery_notification.rs` | 事件通知格式、tokio兼容       |
| StatisticsMessage    | `crates/04-core/shared` | Rust Library | `/src/messages/statistics_message.rs`    | 批量数据传输、压缩            |

### 10.3 外部协议对象项目映射

#### 10.3.1 OpenRTB协议对象分布

| OpenRTB对象 | 所属crate                    | crate类型    | 具体文件路径                  | 实现要点                   |
| ----------- | ---------------------------- | ------------ | ----------------------------- | -------------------------- |
| BidRequest  | `crates/06-external/openrtb` | Rust Library | `/src/models/bid_request.rs`  | OpenRTB 2.5标准实现、serde |
| BidResponse | `crates/06-external/openrtb` | Rust Library | `/src/models/bid_response.rs` | JSON序列化优化、性能       |
| Impression  | `crates/06-external/openrtb` | Rust Library | `/src/models/impression.rs`   | 扩展字段支持、类型安全     |
| User        | `crates/06-external/openrtb` | Rust Library | `/src/models/user.rs`         | 隐私保护实现、数据脱敏     |
| Device      | `crates/06-external/openrtb` | Rust Library | `/src/models/device.rs`       | 设备识别优化、缓存         |

```rust
// 示例：BidRequest OpenRTB对象实现指导
// 文件路径: crates/06-external/openrtb/src/models/bid_request.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BidRequest {
    #[serde(rename = "id")]
    #[validate(length(min = 1))]
    pub id: String,
    
    #[serde(rename = "imp")]
    #[validate(length(min = 1))]
    pub impressions: Vec<Impression>,
    
    #[serde(rename = "user", skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    
    #[serde(rename = "device", skip_serializing_if = "Option::is_none")]
    pub device: Option<Device>,
    
    #[serde(rename = "ext", skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
    
    #[serde(rename = "at", skip_serializing_if = "Option::is_none")]
    pub auction_type: Option<i32>,
    
    #[serde(rename = "tmax", skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<i32>,
    
    #[serde(rename = "cur", skip_serializing_if = "Option::is_none")]
    pub currencies: Option<Vec<String>>,
}

impl BidRequest {
    // 验证方法
    pub fn is_valid(&self) -> bool {
        !self.id.is_empty() && !self.impressions.is_empty()
    }
    
    // 获取第一个展示位
    pub fn first_impression(&self) -> Option<&Impression> {
        self.impressions.first()
    }
    
    // 检查是否支持指定货币
    pub fn supports_currency(&self, currency: &str) -> bool {
        match &self.currencies {
            Some(currencies) => currencies.contains(&currency.to_string()),
            None => currency == "USD", // 默认USD
        }
    }
    
    // 转换为内部广告请求对象
    pub fn to_internal_ad_request(&self) -> Result<crate::domain::AdRequest, String> {
        if !self.is_valid() {
            return Err("BidRequest验证失败".to_string());
        }
        
        // 转换逻辑实现
        todo!("实现BidRequest到AdRequest的转换")
    }
    
    // 获取扩展字段
    pub fn get_extension<T>(&self, key: &str) -> Option<T> 
    where
        T: serde::de::DeserializeOwned,
    {
        self.extensions
            .as_ref()?
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

impl Default for BidRequest {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            impressions: Vec::new(),
            user: None,
            device: None,
            extensions: None,
            auction_type: Some(1), // 第一价格竞拍
            timeout_ms: Some(100),
            currencies: Some(vec!["USD".to_string()]),
        }
    }
}

// 相关类型定义
use crate::models::{Impression, User, Device};
```

#### 10.3.2 VAST协议对象分布

| VAST对象       | 所属crate                 | crate类型    | 具体文件路径                     | 实现要点                      |
| -------------- | ------------------------- | ------------ | -------------------------------- | ----------------------------- |
| VASTDocument   | `crates/06-external/vast` | Rust Library | `/src/models/vast_document.rs`   | XML序列化、DOM解析、quick-xml |
| AdSystem       | `crates/06-external/vast` | Rust Library | `/src/models/ad_system.rs`       | 系统标识、版本管理            |
| Creative       | `crates/06-external/vast` | Rust Library | `/src/models/creative.rs`        | 多媒体资源管理、类型安全      |
| MediaFile      | `crates/06-external/vast` | Rust Library | `/src/models/media_file.rs`      | 编码格式、质量适配            |
| TrackingEvents | `crates/06-external/vast` | Rust Library | `/src/models/tracking_events.rs` | 事件监测、URL管理             |

### 10.4 数据访问层项目映射

#### 10.4.1 数据库实体映射对象分布

| 映射对象             | 所属crate                         | crate类型    | 具体文件路径                              | 实现要点                     |
| -------------------- | --------------------------------- | ------------ | ----------------------------------------- | ---------------------------- |
| AdvertisementEntity  | `crates/05-infrastructure/data-*` | Rust Library | `/src/entities/advertisement_entity.rs`   | SeaORM映射、外键关系、序列化 |
| CampaignEntity       | `crates/05-infrastructure/data-*` | Rust Library | `/src/entities/campaign_entity.rs`        | 复杂类型映射、索引设计       |
| DeliveryRecordEntity | `crates/05-infrastructure/data-*` | Rust Library | `/src/entities/delivery_record_entity.rs` | 时间分区、批量操作、性能优化 |
| UserProfileEntity    | `crates/05-infrastructure/data-*` | Rust Library | `/src/entities/user_profile_entity.rs`    | JSON列、全文索引、隐私保护   |

```rust
// 示例：AdvertisementEntity SeaORM映射实现指导
// 文件路径: crates/05-infrastructure/data-mysql/src/entities/advertisement_entity.rs

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "advertisements")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    
    #[sea_orm(column_type = "String(Some(200))")]
    pub title: String,
    
    pub advertiser_id: i32,
    
    #[sea_orm(column_type = "String(Some(50))")]
    pub status: String, // AdStatus的字符串表示
    
    #[sea_orm(column_type = "Json")]
    pub targeting_policy_json: serde_json::Value,
    
    pub created_at: DateTime<Utc>,
    
    pub updated_at: DateTime<Utc>,
    
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub budget: Option<rust_decimal::Decimal>,
    
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::advertiser_entity::Entity",
        from = "Column::AdvertiserId",
        to = "super::advertiser_entity::Column::Id"
    )]
    Advertiser,
    
    #[sea_orm(has_many = "super::campaign_entity::Entity")]
    Campaigns,
}

impl Related<super::advertiser_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Advertiser.def()
    }
}

impl Related<super::campaign_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Campaigns.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// 转换方法实现
impl Model {
    // 转换为领域实体
    pub fn to_domain_entity(&self) -> Result<crate::domain::Advertisement, String> {
        let targeting_policy = serde_json::from_value(self.targeting_policy_json.clone())
            .map_err(|e| format!("反序列化定向策略失败: {}", e))?;
        
        let status = self.status.parse()
            .map_err(|e| format!("解析广告状态失败: {}", e))?;
        
        let mut ad = crate::domain::Advertisement::new(
            self.title.clone(),
            self.advertiser_id,
            targeting_policy,
        );
        
        ad.set_id(self.id);
        ad.set_status(status);
        ad.set_created_at(self.created_at);
        ad.set_updated_at(self.updated_at);
        
        if let Some(budget) = self.budget {
            ad.set_budget(budget);
        }
        
        Ok(ad)
    }
    
    // 从领域实体创建
    pub fn from_domain_entity(ad: &crate::domain::Advertisement) -> Result<ActiveModel, String> {
        let targeting_policy_json = serde_json::to_value(ad.targeting_policy())
            .map_err(|e| format!("序列化定向策略失败: {}", e))?;
        
        Ok(ActiveModel {
            id: if let Some(id) = ad.id() {
                Set(id)
            } else {
                NotSet
            },
            title: Set(ad.title().to_string()),
            advertiser_id: Set(ad.advertiser_id()),
            status: Set(ad.status().to_string()),
            targeting_policy_json: Set(targeting_policy_json),
            created_at: Set(ad.created_at()),
            updated_at: Set(ad.updated_at()),
            budget: Set(ad.budget()),
            description: Set(ad.description().map(|s| s.to_string())),
        })
    }
}
```

#### 10.4.2 缓存对象分布

| 缓存对象         | 所属crate                                | crate类型    | 具体文件路径                        | 实现要点                       |
| ---------------- | ---------------------------------------- | ------------ | ----------------------------------- | ------------------------------ |
| AdCacheModel     | `crates/05-infrastructure/caching-redis` | Rust Library | `/src/models/ad_cache_model.rs`     | MessagePack序列化、TTL         |
| UserProfileCache | `crates/05-infrastructure/caching-redis` | Rust Library | `/src/models/user_profile_cache.rs` | 压缩存储、过期策略、隐私保护   |
| BudgetCache      | `crates/05-infrastructure/caching-redis` | Rust Library | `/src/models/budget_cache.rs`       | 原子操作、分布式锁、数据一致性 |
| StatisticsCache  | `crates/05-infrastructure/caching-redis` | Rust Library | `/src/models/statistics_cache.rs`   | 滑动窗口、聚合计算、批量更新   |

#### 10.4.3 消息队列对象分布

| 消息对象                | 所属crate                              | crate类型    | 具体文件路径                                 | 实现要点                     |
| ----------------------- | -------------------------------------- | ------------ | -------------------------------------------- | ---------------------------- |
| DeliveryEventMessage    | `crates/05-infrastructure/messaging-*` | Rust Library | `/src/messages/delivery_event_message.rs`    | 事件序列化、顺序保证、幂等性 |
| BudgetUpdateMessage     | `crates/05-infrastructure/messaging-*` | Rust Library | `/src/messages/budget_update_message.rs`     | 幂等性、重试机制、原子性     |
| AuditLogMessage         | `crates/05-infrastructure/messaging-*` | Rust Library | `/src/messages/audit_log_message.rs`         | 安全传输、完整性校验、加密   |
| StatisticsUpdateMessage | `crates/05-infrastructure/messaging-*` | Rust Library | `/src/messages/statistics_update_message.rs` | 批量处理、压缩传输、聚合     |

### 10.5 共享组件项目映射

#### 10.5.1 枚举和常量分布

| 枚举/常量类型   | 所属crate               | crate类型    | 具体文件路径                          | 实现要点                     |
| --------------- | ----------------------- | ------------ | ------------------------------------- | ---------------------------- |
| AdStatus        | `crates/04-core/shared` | Rust Library | `/src/enums/ad_status.rs`             | 状态机实现、转换验证、序列化 |
| CampaignStatus  | `crates/04-core/shared` | Rust Library | `/src/enums/campaign_status.rs`       | 生命周期管理、状态转换       |
| BiddingStrategy | `crates/04-core/shared` | Rust Library | `/src/enums/bidding_strategy.rs`      | 竞价算法标识、策略模式       |
| AdSizeConstants | `crates/04-core/shared` | Rust Library | `/src/constants/ad_size_constants.rs` | IAB标准尺寸、类型安全        |
| ErrorCodes      | `crates/04-core/shared` | Rust Library | `/src/constants/error_codes.rs`       | 统一错误编码、错误链         |

```rust
// 示例：AdStatus枚举实现指导
// 文件路径: crates/04-core/shared/src/enums/ad_status.rs

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdStatus {
    /// 草稿状态
    Draft = 0,
    /// 待审核
    PendingReview = 1,
    /// 审核通过
    Approved = 2,
    /// 审核拒绝
    Rejected = 3,
    /// 投放中
    Active = 4,
    /// 暂停
    Paused = 5,
    /// 已结束
    Completed = 6,
}

impl AdStatus {
    /// 检查状态转换是否有效
    pub fn can_transition_to(&self, target: AdStatus) -> bool {
        use AdStatus::*;
        match (self, target) {
            (Draft, PendingReview) => true,
            (PendingReview, Approved | Rejected) => true,
            (Approved, Active | Paused) => true,
            (Active, Paused | Completed) => true,
            (Paused, Active | Completed) => true,
            (Rejected, Draft) => true, // 可以重新编辑
            _ => false,
        }
    }
    
    /// 获取状态描述
    pub fn description(&self) -> &'static str {
        match self {
            AdStatus::Draft => "草稿状态",
            AdStatus::PendingReview => "待审核",
            AdStatus::Approved => "审核通过",
            AdStatus::Rejected => "审核拒绝",
            AdStatus::Active => "投放中",
            AdStatus::Paused => "暂停",
            AdStatus::Completed => "已结束",
        }
    }
    
    /// 获取所有可能的后续状态
    pub fn possible_transitions(&self) -> Vec<AdStatus> {
        use AdStatus::*;
        match self {
            Draft => vec![PendingReview],
            PendingReview => vec![Approved, Rejected],
            Approved => vec![Active, Paused],
            Active => vec![Paused, Completed],
            Paused => vec![Active, Completed],
            Rejected => vec![Draft],
            Completed => vec![], // 终态
        }
    }
    
    /// 检查是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, AdStatus::Completed)
    }
    
    /// 检查是否为活跃状态
    pub fn is_active(&self) -> bool {
        matches!(self, AdStatus::Active)
    }
}

impl Display for AdStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.description())
    }
}

impl FromStr for AdStatus {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(AdStatus::Draft),
            "pendingreview" | "pending_review" => Ok(AdStatus::PendingReview),
            "approved" => Ok(AdStatus::Approved),
            "rejected" => Ok(AdStatus::Rejected),
            "active" => Ok(AdStatus::Active),
            "paused" => Ok(AdStatus::Paused),
            "completed" => Ok(AdStatus::Completed),
            _ => Err(format!("无效的广告状态: {}", s)),
        }
    }
}

impl Default for AdStatus {
    fn default() -> Self {
        AdStatus::Draft
    }
}

// 状态转换结果
#[derive(Debug, Clone)]
pub struct StatusTransition {
    pub from: AdStatus,
    pub to: AdStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reason: Option<String>,
}

impl StatusTransition {
    pub fn new(from: AdStatus, to: AdStatus, reason: Option<String>) -> Result<Self, String> {
        if !from.can_transition_to(to) {
            return Err(format!("无效的状态转换: {} -> {}", from, to));
        }
        
        Ok(Self {
            from,
            to,
            timestamp: chrono::Utc::now(),
            reason,
        })
    }
}
```

#### 10.5.2 扩展方法分布

| 扩展trait/模块       | 所属crate               | crate类型    | 具体文件路径                               | 实现要点                       |
| -------------------- | ----------------------- | ------------ | ------------------------------------------ | ------------------------------ |
| DateTimeExtensions   | `crates/04-core/shared` | Rust Library | `/src/extensions/datetime_extensions.rs`   | 时区处理、格式化、chrono集成   |
| CollectionExtensions | `crates/04-core/shared` | Rust Library | `/src/extensions/collection_extensions.rs` | 分页、过滤、聚合、迭代器       |
| StringExtensions     | `crates/04-core/shared` | Rust Library | `/src/extensions/string_extensions.rs`     | 验证、格式化、编码、正则表达式 |
| JsonExtensions       | `crates/04-core/shared` | Rust Library | `/src/extensions/json_extensions.rs`       | 序列化、安全解析、serde集成    |

### 10.6 算法和服务对象项目映射

#### 10.6.1 算法实现对象分布

| 算法对象          | 所属crate                      | crate类型    | 具体文件路径                            | 实现要点                     |
| ----------------- | ------------------------------ | ------------ | --------------------------------------- | ---------------------------- |
| TargetingMatcher  | `crates/02-services/targeting` | Binary Crate | `/src/algorithms/targeting_matcher.rs`  | 匹配算法、性能优化、并行计算 |
| BiddingCalculator | `crates/02-services/bidding`   | Binary Crate | `/src/algorithms/bidding_calculator.rs` | 竞价算法、质量评估、实时计算 |
| BudgetController  | `crates/02-services/campaign`  | Binary Crate | `/src/algorithms/budget_controller.rs`  | 预算控制、令牌桶、原子操作   |
| RecallEngine      | `crates/02-services/ad-engine` | Binary Crate | `/src/algorithms/recall_engine.rs`      | 召回算法、多路召回、缓存优化 |

#### 10.6.2 服务trait和实现分布

| 服务trait         | trait定义crate               | 实现crate                      | trait文件路径                          | 实现文件路径                           |
| ----------------- | ---------------------------- | ------------------------------ | -------------------------------------- | -------------------------------------- |
| AdDeliveryService | `crates/04-core/application` | `crates/02-services/ad-engine` | `/src/services/ad_delivery_service.rs` | `/src/services/ad_delivery_service.rs` |
| TargetingService  | `crates/04-core/application` | `crates/02-services/targeting` | `/src/services/targeting_service.rs`   | `/src/services/targeting_service.rs`   |
| BiddingService    | `crates/04-core/application` | `crates/02-services/bidding`   | `/src/services/bidding_service.rs`     | `/src/services/bidding_service.rs`     |
| CampaignService   | `crates/04-core/application` | `crates/02-services/campaign`  | `/src/services/campaign_service.rs`    | `/src/services/campaign_service.rs`    |

### 10.7 配置和设置对象项目映射

#### 10.7.1 配置对象分布

| 配置对象        | 所属crate                            | crate类型    | 具体文件路径                              | 实现要点                     |
| --------------- | ------------------------------------ | ------------ | ----------------------------------------- | ---------------------------- |
| AdEngineOptions | `crates/02-services/ad-engine`       | Binary Crate | `/src/configuration/ad_engine_options.rs` | 强类型配置、验证、环境变量   |
| BiddingOptions  | `crates/02-services/bidding`         | Binary Crate | `/src/configuration/bidding_options.rs`   | 算法参数、策略配置、动态调整 |
| CacheOptions    | `crates/05-infrastructure/caching-*` | Rust Library | `/src/configuration/cache_options.rs`     | 缓存策略、过期设置、内存管理 |
| DatabaseOptions | `crates/05-infrastructure/data-*`    | Rust Library | `/src/configuration/database_options.rs`  | 连接字符串、性能参数、连接池 |

```rust
// 示例：AdEngineOptions配置对象实现指导
// 文件路径: crates/02-services/ad-engine/src/configuration/ad_engine_options.rs

use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AdEngineOptions {
    #[validate(range(min = 1, max = 10000))]
    pub max_concurrent_requests: u32,
    
    #[validate(range(min = 50, max = 5000))]
    pub request_timeout_ms: u32,
    
    #[validate(length(min = 1))]
    pub supported_formats: Vec<String>,
    
    #[validate]
    pub budget_control: BudgetControlOptions,
    
    #[validate]
    pub performance: PerformanceOptions,
    
    #[serde(default = "default_config_section")]
    pub config_section: String,
}

impl Default for AdEngineOptions {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 5000,
            request_timeout_ms: 100,
            supported_formats: vec![
                "banner".to_string(),
                "video".to_string(),
                "native".to_string(),
            ],
            budget_control: BudgetControlOptions::default(),
            performance: PerformanceOptions::default(),
            config_section: "AdEngine".to_string(),
        }
    }
}

impl AdEngineOptions {
    // 从环境变量加载配置
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::Environment::with_prefix("AD_ENGINE"))
            .build()?;
        
        settings.try_deserialize()
    }
    
    // 验证配置
    pub fn validate_config(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // 基础验证
        if let Err(validation_errors) = self.validate() {
            for (field, field_errors) in validation_errors.field_errors() {
                for error in field_errors {
                    errors.push(format!("字段 {}: {}", field, error.message.as_ref().unwrap_or(&"验证失败".into())));
                }
            }
        }
        
        // 业务规则验证
        if self.request_timeout_ms < 50 {
            errors.push("请求超时时间不能小于50毫秒".to_string());
        }
        
        if self.supported_formats.is_empty() {
            errors.push("必须支持至少一种广告格式".to_string());
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    // 获取请求超时时间
    pub fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.request_timeout_ms as u64)
    }
    
    // 检查是否支持指定格式
    pub fn supports_format(&self, format: &str) -> bool {
        self.supported_formats.iter().any(|f| f.eq_ignore_ascii_case(format))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BudgetControlOptions {
    pub enable_real_time_check: bool,
    
    #[validate(range(min = 100, max = 10000))]
    pub check_interval_ms: u32,
    
    #[validate(range(min = 0.0, max = 50.0))]
    pub safety_margin_percent: f64,
}

impl Default for BudgetControlOptions {
    fn default() -> Self {
        Self {
            enable_real_time_check: true,
            check_interval_ms: 1000,
            safety_margin_percent: 5.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PerformanceOptions {
    pub enable_caching: bool,
    
    #[validate(range(min = 10, max = 3600))]
    pub cache_ttl_seconds: u32,
    
    pub enable_compression: bool,
    
    #[validate(range(min = 1, max = 1000))]
    pub worker_threads: Option<u32>,
}

impl Default for PerformanceOptions {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl_seconds: 300,
            enable_compression: true,
            worker_threads: None, // 使用系统默认
        }
    }
}

fn default_config_section() -> String {
    "AdEngine".to_string()
}

// 配置加载器
pub struct ConfigLoader;

impl ConfigLoader {
    // 从多个源加载配置
    pub fn load() -> Result<AdEngineOptions, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config/ad_engine").required(false))
            .add_source(config::Environment::with_prefix("AD_ENGINE"))
            .build()?;
        
        let options: AdEngineOptions = settings.try_deserialize()?;
        
        // 验证配置
        if let Err(errors) = options.validate_config() {
            return Err(config::ConfigError::Message(format!("配置验证失败: {:?}", errors)));
        }
        
        Ok(options)
    }
}
```

### 10.8 数据库映射和迁移项目配置

#### 10.8.1 数据库连接配置

| 数据库上下文      | 所属crate                         | crate类型    | 配置文件路径                          | 实现要点                       |
| ----------------- | --------------------------------- | ------------ | ------------------------------------- | ------------------------------ |
| AdSystemDatabase  | `crates/05-infrastructure/data-*` | Rust Library | `/src/contexts/ad_system_database.rs` | SeaORM连接、实体配置、性能优化 |
| AnalyticsDatabase | `crates/05-infrastructure/data-*` | Rust Library | `/src/contexts/analytics_database.rs` | 读写分离、分区表、时间序列     |
| AuditDatabase     | `crates/05-infrastructure/data-*` | Rust Library | `/src/contexts/audit_database.rs`     | 审计日志、只写模式、数据保留   |

#### 10.8.2 数据库迁移管理

| 迁移类型 | 所属crate                         | crate类型    | 迁移文件路径             | 管理策略                       |
| -------- | --------------------------------- | ------------ | ------------------------ | ------------------------------ |
| 结构迁移 | `crates/05-infrastructure/data-*` | Rust Library | `/migrations/structure/` | 版本化管理、自动执行、回滚支持 |
| 数据迁移 | `crates/09-tools/data-migration`  | Binary Crate | `/src/migrations/data/`  | 手动执行、数据验证、进度跟踪   |
| 索引迁移 | `crates/05-infrastructure/data-*` | Rust Library | `/migrations/indexes/`   | 性能监控、在线执行、影响评估   |
                
| 结构迁移 | `crates/05-infrastructure/data-*`   | Rust Library     | `/migrations/structure/`         | 版本化管理、自动执行、回滚支持 |
| 数据迁移 | `crates/09-tools/data-migration`    | Binary Crate     | `/src/migrations/data/`          | 手动执行、数据验证、进度跟踪   |
| 索引迁移 | `crates/05-infrastructure/data-*`   | Rust Library     | `/migrations/indexes/`           | 性能监控、在线执行、影响评估   |

## 10.9 Cargo工作空间总结

### 10.9.1 核心优势

1. **内存安全**：Rust编译时保证内存安全，避免空指针和数据竞争问题
2. **零成本抽象**：trait系统提供高度抽象的同时保持运行时性能
3. **并发安全**：所有权模型和类型系统确保并发安全
4. **模块化设计**：每个crate职责单一，依赖关系清晰
5. **跨平台支持**：编译为原生代码，支持多种操作系统和架构

### 10.9.2 开发指导原则

1. **领域驱动设计**：严格按照DDD原则组织代码结构
2. **依赖倒置**：通过trait定义抽象层，实现依赖注入
3. **错误处理**：统一使用Result类型进行错误处理
4. **异步编程**：基于Tokio运行时实现高并发处理
5. **测试驱动**：每个crate都包含完整的单元测试和集成测试

### 10.9.3 性能优化策略

1. **编译时优化**：利用Rust编译器的优化能力
2. **零拷贝设计**：尽可能使用借用和引用，避免不必要的内存复制
3. **缓存友好**：数据结构设计考虑CPU缓存特性
4. **并行计算**：使用rayon等库实现数据并行处理
5. **内存池**：对于高频分配的对象使用对象池模式
