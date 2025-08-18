use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use sea_orm_migration::prelude::*;

use crate::database::entity::{prelude::TUsers, t_users};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("t_users")
                    .if_not_exists()
                    .col(
                        ColumnDef::new("id")
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new("username").string().not_null())
                    .col(ColumnDef::new("password").string().not_null())
                    .col(ColumnDef::new("created_at").date_time().not_null())
                    .col(ColumnDef::new("updated_at").date_time().not_null())
                    .col(ColumnDef::new("deleted_at").date_time().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table("t_settings")
                    .if_not_exists()
                    .col(
                        ColumnDef::new("id")
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new("name").string().not_null())
                    .col(ColumnDef::new("sequence").integer().not_null())
                    .col(ColumnDef::new("description").string().null())
                    .col(ColumnDef::new("value").string().not_null())
                    .col(ColumnDef::new("created_at").date_time().not_null())
                    .col(ColumnDef::new("updated_at").date_time().not_null())
                    .col(ColumnDef::new("deleted_at").date_time().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table("t_logs")
                    .if_not_exists()
                    .col(
                        ColumnDef::new("id")
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new("user_id").big_integer().not_null())
                    .col(ColumnDef::new("action").string().not_null())
                    .col(ColumnDef::new("details").json().not_null())
                    .col(ColumnDef::new("created_at").date_time().not_null())
                    .col(ColumnDef::new("updated_at").date_time().not_null())
                    .col(ColumnDef::new("deleted_at").date_time().null())
                    .to_owned(),
            )
            .await?;

        match TUsers::find()
            .filter(t_users::Column::Username.eq("admin"))
            .one(manager.get_connection())
            .await
        {
            Ok(Some(_)) => {}
            Ok(None) => {
                let password = bcrypt::hash("admin", bcrypt::DEFAULT_COST).unwrap();
                let user = t_users::ActiveModel {
                    username: Set("admin".to_string()),
                    password: Set(password),
                    created_at: Set(chrono::Local::now()),
                    updated_at: Set(chrono::Local::now()),
                    ..Default::default()
                };
                TUsers::insert(user).exec(manager.get_connection()).await?;
            }
            Err(e) => {
                return Err(e);
            }
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
