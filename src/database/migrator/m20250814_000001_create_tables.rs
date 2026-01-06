use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use sea_orm_migration::prelude::*;
#[cfg(feature = "postgres")]
use sea_orm::Statement;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use crate::chrono::Local;
use crate::database::entity::{prelude::TUsers, t_settings, t_users};
#[cfg(any(feature = "sqlite", feature = "mysql"))]
use sea_orm::prelude::DateTimeWithTimeZone;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        {
            manager
                .create_table(
                    Table::create()
                        .table("t_users")
                        .if_not_exists()
                        .col(ColumnDef::new("id").uuid().not_null().primary_key())
                        .col(ColumnDef::new("username").string().not_null())
                        .col(ColumnDef::new("password").string().not_null())
                        .col(
                            ColumnDef::new("created_at")
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new("updated_at")
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new("deleted_at")
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .name("idx-users-username")
                        .table("t_users")
                        .col(t_users::Column::Username)
                        .unique()
                        .to_owned(),
                )
                .await?;
        }

        {
            manager
                .create_table(
                    Table::create()
                        .table("t_settings")
                        .if_not_exists()
                        .col(ColumnDef::new("id").uuid().not_null().primary_key())
                        .col(ColumnDef::new("key").string().not_null())
                        .col(ColumnDef::new("value").json().not_null())
                        .col(
                            ColumnDef::new("created_at")
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new("updated_at")
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new("deleted_at")
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .index(
                            Index::create()
                                .name("idx_t_settings_key_deleted_at_unique")
                                .col("key")
                                .col("deleted_at")
                                .unique(),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("idx-settings-key")
                        .table("t_settings")
                        .col(t_settings::Column::Key)
                        .to_owned(),
                )
                .await?;
        }

        {
            #[cfg(any(feature = "sqlite", feature = "mysql"))]
            {
                manager
                    .create_table(
                        Table::create()
                            .table("t_logs")
                            .if_not_exists()
                            .col(ColumnDef::new("id").uuid().not_null().primary_key())
                            .col(ColumnDef::new("user_id").uuid())
                            .col(ColumnDef::new("action").string().not_null())
                            .col(ColumnDef::new("details").json_binary().not_null())
                            .col(
                                ColumnDef::new("created_at")
                                    .timestamp_with_time_zone()
                                    .not_null()
                                    .default(DateTimeWithTimeZone::from(
                                        Local::now().fixed_offset(),
                                    )),
                            )
                            .col(
                                ColumnDef::new("updated_at")
                                    .timestamp_with_time_zone()
                                    .not_null()
                                    .default(DateTimeWithTimeZone::from(
                                        Local::now().fixed_offset(),
                                    )),
                            )
                            .col(ColumnDef::new("deleted_at").timestamp_with_time_zone())
                            .to_owned(),
                    )
                    .await?;
            }
            #[cfg(feature = "postgres")]
            {
                let create_sql = r#"
CREATE TABLE IF NOT EXISTS t_logs (
    id UUID NOT NULL,
    user_id UUID,
    action TEXT NOT NULL,
    details JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ NULL,
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);
            "#;
                manager
                    .get_connection()
                    .execute(Statement::from_string(
                        manager.get_database_backend(),
                        create_sql.to_owned(),
                    ))
                    .await?;
                let partition_trigger = r#"
CREATE OR REPLACE FUNCTION create_t_logs_current_month_partition()
RETURNS void AS $$
DECLARE
    current_month_partition TEXT;  -- 存储当前月份分区表的名称
    start_of_month TIMESTAMPTZ;   -- 存储当前月份的开始时间戳
    start_of_next_month TIMESTAMPTZ; -- 存储下个月份的开始时间戳
BEGIN
    -- 动态生成当前月份分区表名，格式为 t_logs_YYYY_MM
    current_month_partition := 't_logs_' || TO_CHAR(CURRENT_DATE, 'YYYY_MM');
    
    -- 计算当前月的第一天（00:00:00）
    start_of_month := DATE_TRUNC('month', CURRENT_DATE);
    -- 计算下个月的第一天，作为当前月分区的上限（不包含）
    start_of_next_month := start_of_month + INTERVAL '1 month';
    
    -- 检查分区表是否已存在
    IF NOT EXISTS (
        SELECT 1
        FROM pg_tables 
        WHERE schemaname = 'public' 
        AND tablename = current_month_partition
    ) THEN
        -- 动态执行SQL创建分区表，仅当不存在时才创建
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF t_logs 
            FOR VALUES FROM (%L) TO (%L)',
            current_month_partition, 
            start_of_month, 
            start_of_next_month
        );
        -- 输出创建成功的日志信息
        RAISE NOTICE '分区表 % 创建成功，时间范围: % 至 %', 
            current_month_partition, 
            start_of_month, 
            start_of_next_month;
    ELSE
        -- 输出分区已存在的日志信息
        RAISE NOTICE '分区表 % 已存在，无需创建', current_month_partition;
    END IF;
END;
$$ LANGUAGE plpgsql;
            "#;
                manager
                    .get_connection()
                    .execute(Statement::from_string(
                        manager.get_database_backend(),
                        partition_trigger.to_owned(),
                    ))
                    .await?;

                let call_trigger = r#"
SELECT create_t_logs_current_month_partition();
            "#;

                manager
                    .get_connection()
                    .execute(Statement::from_string(
                        manager.get_database_backend(),
                        call_trigger.to_owned(),
                    ))
                    .await?;
            }
        }

        {
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
                        ..Default::default()
                    };
                    TUsers::insert(user).exec(manager.get_connection()).await?;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        {
            match TUsers::find()
                .filter(t_users::Column::Username.eq("sys"))
                .one(manager.get_connection())
                .await
            {
                Ok(Some(_)) => {}
                Ok(None) => {
                    let password = bcrypt::hash("sys", bcrypt::DEFAULT_COST).unwrap();
                    let user = t_users::ActiveModel {
                        username: Set("sys".to_string()),
                        password: Set(password),
                        ..Default::default()
                    };
                    TUsers::insert(user).exec(manager.get_connection()).await?;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
