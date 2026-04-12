use std::str::FromStr;

use crate::utils::datetime::{to_local_time, to_local_time_option};
use chrono::Local;
use sea_orm::{Set, entity::prelude::*};
use sea_orm_migration::prelude::ValueTypeErr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ROI purpose/type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RoiPurpose {
    /// Detection region - where inspection occurs
    Detection,
    /// Alignment reference - for position calibration
    Alignment,
    /// Exclusion region - ignore this area
    Exclusion,
}

impl RoiPurpose {
    /// Get display name in Chinese
    pub fn display_name(&self) -> &'static str {
        match self {
            RoiPurpose::Detection => "检测区域",
            RoiPurpose::Alignment => "定位参考",
            RoiPurpose::Exclusion => "排除区域",
        }
    }
}

#[derive(Debug)]
pub struct ParseRoiPurposeError;

impl FromStr for RoiPurpose {
    type Err = ParseRoiPurposeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Detection" => Ok(RoiPurpose::Detection),
            "Alignment" => Ok(RoiPurpose::Alignment),
            "Exclusion" => Ok(RoiPurpose::Exclusion),
            _ => Err(ParseRoiPurposeError),
        }
    }
}

impl From<RoiPurpose> for sea_orm::Value {
    fn from(source: RoiPurpose) -> Self {
        match source {
            RoiPurpose::Detection => "Detection".into(),
            RoiPurpose::Alignment => "Alignment".into(),
            RoiPurpose::Exclusion => "Exclusion".into(),
        }
    }
}

impl sea_orm::TryGetable for RoiPurpose {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, index)
            .map(|v| RoiPurpose::from_str(&v).unwrap_or(RoiPurpose::Detection))
    }
}

impl sea_orm::sea_query::ValueType for RoiPurpose {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v)
            .map(|v| RoiPurpose::from_str(&v).unwrap_or(RoiPurpose::Detection))
    }

    fn type_name() -> String {
        stringify!(RoiPurpose).to_owned()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }

    fn column_type() -> ColumnType {
        sea_orm::sea_query::ColumnType::String(StringLen::N(20))
    }
}

impl sea_orm::sea_query::Nullable for RoiPurpose {
    fn null() -> Value {
        <String as sea_orm::sea_query::Nullable>::null()
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_station_rois")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    /// Station ID (foreign key to t_inspection_stations)
    pub station_id: Uuid,

    /// ROI name
    #[sea_orm(column_type = "String(StringLen::N(100))")]
    pub name: String,

    /// Shape definition (JSON: RoiShape)
    #[sea_orm(column_type = "Json")]
    pub shape: Json,

    /// Purpose of this ROI
    #[sea_orm(default_value = "Detection")]
    pub purpose: RoiPurpose,

    /// Whether this ROI is enabled
    #[sea_orm(default_value = "true")]
    pub enabled: bool,

    #[serde(serialize_with = "to_local_time")]
    pub created_at: DateTimeWithTimeZone,

    #[serde(serialize_with = "to_local_time")]
    pub updated_at: DateTimeWithTimeZone,

    #[serde(serialize_with = "to_local_time_option")]
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::t_inspection_stations::Entity",
        from = "Column::StationId",
        to = "super::t_inspection_stations::Column::Id"
    )]
    Station,
}

impl Related<super::t_inspection_stations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Station.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7()),
            station_id: Set(Uuid::nil()),
            purpose: Set(RoiPurpose::Detection),
            enabled: Set(true),
            shape: Set(Json::Null),
            ..ActiveModelTrait::default()
        }
    }

    fn before_save<'life0, 'async_trait, C>(
        mut self,
        _db: &'life0 C,
        _insert: bool,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, DbErr>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        C: ConnectionTrait,
        C: 'async_trait,
        'life0: 'async_trait,
        Self: ::core::marker::Send + 'async_trait,
    {
        Box::pin(async move {
            self.updated_at = Set(DateTimeWithTimeZone::from(Local::now()));
            if self.created_at.is_not_set() {
                self.created_at = Set(DateTimeWithTimeZone::from(Local::now()));
            }
            Ok(self)
        })
    }
}