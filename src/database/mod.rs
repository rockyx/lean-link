use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult, PaginatorTrait,
    QueryFilter, QueryOrder, UpdateResult, prelude::Json, sea_query::OnConflict,
};

use crate::database::entity::{
    PageResult,
    prelude::{TLogs, TSettings, TUsers},
    t_logs, t_settings, t_users,
};
use std::str::FromStr;

pub mod entity;
pub mod migrator;

pub trait DbHelperTrait {}

#[derive(Clone)]
pub struct DbHelper {
    connection: DatabaseConnection,
}

impl DbHelper {
    pub fn new(connection: DatabaseConnection) -> Self {
        Self {
            connection: connection,
        }
    }

    pub async fn find_user_by_name(
        &self,
        username: String,
    ) -> Result<Option<t_users::Model>, DbErr> {
        TUsers::find()
            .filter(
                t_users::Column::Username
                    .eq(username)
                    .and(t_users::Column::DeletedAt.is_null()),
            )
            .one(&self.connection)
            .await
    }

    pub async fn insert_log(
        &self,
        user_id: i64,
        action: String,
        details: Json,
    ) -> Result<InsertResult<t_logs::ActiveModel>, DbErr> {
        TLogs::insert(t_logs::ActiveModel {
            id: ActiveValue::not_set(),
            user_id: ActiveValue::set(user_id),
            action: ActiveValue::set(action),
            details: ActiveValue::set(details),
            created_at: ActiveValue::not_set(),
            deleted_at: ActiveValue::not_set(),
            updated_at: ActiveValue::not_set(),
        })
        .exec(&self.connection)
        .await
    }

    pub async fn clear_all_logs(&self) -> Result<UpdateResult, DbErr> {
        TLogs::update_many()
            .set(t_logs::ActiveModel {
                deleted_at: ActiveValue::set(Some(chrono::Local::now())),
                ..Default::default()
            })
            .filter(t_logs::Column::DeletedAt.is_null())
            .exec(&self.connection)
            .await
    }

    pub async fn setting_get_x<T>(&self, name: &str) -> Result<T, DbErr>
    where
        T: FromStr + Default,
    {
        let model = TSettings::find()
            .filter(t_settings::Column::Name.eq(name))
            .filter(t_settings::Column::Sequence.eq(1))
            .one(&self.connection)
            .await?;

        Ok(match model {
            None => T::default(),
            Some(config) => T::from_str(&config.value).unwrap_or_default(),
        })
    }

    pub async fn setting_set_x<T: ToString>(
        &self,
        name: &str,
        desc: &str,
        value: T,
    ) -> Result<(), DbErr> {
        TSettings::insert(t_settings::ActiveModel {
            id: ActiveValue::not_set(),
            name: ActiveValue::set(name.into()),
            sequence: ActiveValue::set(1.into()),
            description: ActiveValue::set(Some(desc.into())),
            value: ActiveValue::set(value.to_string()),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([t_settings::Column::Name, t_settings::Column::Sequence])
                .update_column(t_settings::Column::Value)
                .to_owned(),
        )
        .exec(&self.connection)
        .await?;

        Ok(())
    }

    pub async fn page_logs(
        &self,
        mut page_index: u64,
        mut page_size: u64,
    ) -> Result<PageResult<t_logs::Model>, DbErr> {
        if page_index <= 0 {
            page_index = 1;
        }

        if page_size <= 0 {
            page_size = 10;
        }

        let page_find = TLogs::find()
            .filter(t_logs::Column::DeletedAt.is_null())
            .order_by_desc(t_logs::Column::CreatedAt)
            .paginate(&self.connection, page_size);

        let mut page = PageResult::<t_logs::Model> {
            records: vec![],
            page_index,
            page_size,
            total_count: 0,
            pages: 0,
        };

        match page_find.num_items_and_pages().await {
            Ok(v) => {
                page.total_count = v.number_of_items;
                page.pages = v.number_of_pages;
            }
            Err(e) => {
                return Err(e);
            }
        };

        match page_find.fetch_page(page_index - 1).await {
            Ok(items) => {
                page.records = items;
                Ok(page)
            }
            Err(e) => Err(e),
        }
    }
}

pub async fn init_connection(db_url: &str) -> Result<DatabaseConnection, DbErr> {
    sea_orm::Database::connect(db_url).await
}
