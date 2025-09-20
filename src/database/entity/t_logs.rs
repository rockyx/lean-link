use crate::utils::datetime::{to_local_time, to_local_time_option};
use chrono::Local;
use sea_orm::{Set, entity::prelude::*};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "t_logs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub details: Json,
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
