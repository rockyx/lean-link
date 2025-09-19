use crate::utils::datetime::{to_local_time, to_local_time_option};
use chrono::Local;
use sea_orm::{Set, entity::prelude::*};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "t_users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique_index)]
    pub username: String,
    pub password: String,
    #[serde(serialize_with = "to_local_time", rename = "createdAt")]
    pub created_at: DateTimeWithTimeZone,
    #[serde(serialize_with = "to_local_time", rename = "updatedAt")]
    pub updated_at: DateTimeWithTimeZone,
    #[serde(serialize_with = "to_local_time_option", rename = "deletedAt")]
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7()),
            created_at: Set(Local::now().fixed_offset()),
            updated_at: Set(Local::now().fixed_offset()),
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
