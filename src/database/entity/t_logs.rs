use std::str::FromStr;

use crate::utils::datetime::{to_local_time, to_local_time_option};
use chrono::Local;
use sea_orm::{Set, entity::prelude::*};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug)]
pub struct ParseLogLevelError;

impl FromStr for LogLevel {
    type Err = ParseLogLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Debug" => Ok(LogLevel::Debug),
            "Info" => Ok(LogLevel::Info),
            "Warning" => Ok(LogLevel::Warning),
            "Error" => Ok(LogLevel::Error),
            _ => Err(ParseLogLevelError),
        }
    }
}

#[automatically_derived]
impl std::convert::From<LogLevel> for sea_orm::Value {
    fn from(source: LogLevel) -> Self {
        match source {
            LogLevel::Debug => "Debug".into(),
            LogLevel::Info => "Info".into(),
            LogLevel::Warning => "Warning".into(),
            LogLevel::Error => "Error".into(),
        }
    }
}

#[automatically_derived]
impl sea_orm::TryGetable for LogLevel {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, index)
            .map(|v| LogLevel::from_str(&v).unwrap_or(LogLevel::Info))
    }
}

#[automatically_derived]
impl sea_orm::sea_query::ValueType for LogLevel {
    fn try_from(v: Value) -> Result<Self, sea_orm_migration::prelude::ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v)
            .map(|v| LogLevel::from_str(&v).unwrap_or(LogLevel::Info))
    }

    fn type_name() -> String {
        stringify!(LogLevel).to_owned()
    }

    fn array_type() -> sea_orm_migration::prelude::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }

    fn column_type() -> ColumnType {
        sea_orm::sea_query::ColumnType::String(StringLen::N(10))
    }
}

#[automatically_derived]
impl sea_orm::sea_query::Nullable for LogLevel {
    fn null() -> Value {
        <String as sea_orm::sea_query::Nullable>::null()
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "t_logs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub details: Json,
    pub level: LogLevel,
    #[serde(serialize_with = "to_local_time")]
    pub created_at: DateTimeWithTimeZone,
    #[serde(serialize_with = "to_local_time")]
    pub updated_at: DateTimeWithTimeZone,
    #[serde(serialize_with = "to_local_time_option")]
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7()),
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
            self.updated_at = Set(Local::now().fixed_offset());
            Ok(self)
        })
    }
}
