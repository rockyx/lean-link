use crate::utils::datetime::{to_local_time, to_local_time_option};
use chrono::Local;
use sea_orm::{Set, entity::prelude::*};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "t_settings")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, default = "Uuid::now_v7()")]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub key: String,
    pub value: Json,
    #[serde(serialize_with = "to_local_time")]
    pub created_at: DateTimeWithTimeZone,
    #[serde(serialize_with = "to_local_time")]
    pub updated_at: DateTimeWithTimeZone,
    #[serde(serialize_with = "to_local_time_option")]
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7()),
            created_at: Set(DateTimeWithTimeZone::from(Local::now().fixed_offset())),
            updated_at: Set(DateTimeWithTimeZone::from(Local::now().fixed_offset())),
            ..ActiveModelTrait::default()
        }
    }

    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        tracing::info!("before save");
        if insert {
            self.id = Set(Uuid::now_v7());
            self.created_at = Set(DateTimeWithTimeZone::from(Local::now()));
            self.updated_at = Set(DateTimeWithTimeZone::from(Local::now()));
        }
        else {
            self.updated_at = Set(DateTimeWithTimeZone::from(Local::now()));
        }
        Ok(self)
    }
}
