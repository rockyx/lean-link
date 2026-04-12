use crate::database::entity::{PageResult, prelude::TModbusConfigs, t_modbus_configs};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

pub async fn insert_modbus_config(
    conn: &DatabaseConnection,
    config: t_modbus_configs::ActiveModel,
) -> Result<InsertResult<t_modbus_configs::ActiveModel>, DbErr> {
    TModbusConfigs::insert(config).exec(conn).await
}

pub async fn find_modbus_config_by_id(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<t_modbus_configs::Model>, DbErr> {
    TModbusConfigs::find_by_id(id).one(conn).await
}

pub async fn find_all_modbus_configs(
    conn: &DatabaseConnection,
) -> Result<Vec<t_modbus_configs::Model>, DbErr> {
    TModbusConfigs::find()
        .all(conn)
        .await
}

pub async fn find_modbus_configs_by_enabled(
    conn: &DatabaseConnection,
    enabled: bool,
) -> Result<Vec<t_modbus_configs::Model>, DbErr> {
    TModbusConfigs::find()
        .filter(t_modbus_configs::Column::Enabled.eq(enabled))
        .all(conn)
        .await
}

pub async fn page_modbus_configs(
    conn: &DatabaseConnection,
    mut page_index: u64,
    mut page_size: u64,
) -> Result<PageResult<t_modbus_configs::Model>, DbErr> {
    if page_index == 0 {
        page_index = 1;
    }

    if page_size == 0 {
        page_size = 10;
    }

    let page_find = TModbusConfigs::find()
        .order_by_asc(t_modbus_configs::Column::Id)
        .paginate(conn, page_size);

    let mut page = PageResult::<t_modbus_configs::Model> {
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

pub async fn update_modbus_config(
    conn: &DatabaseConnection,
    id: Uuid,
    config: t_modbus_configs::ActiveModel,
) -> Result<Option<t_modbus_configs::Model>, DbErr> {
    let existing = TModbusConfigs::find_by_id(id).one(conn).await?;
    if existing.is_none() {
        return Ok(None);
    }

    let mut config = config;
    config.id = ActiveValue::set(id);
    
    TModbusConfigs::update(config).exec(conn).await.map(Some)
}

pub async fn delete_modbus_config(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<bool, DbErr> {
    let result = TModbusConfigs::delete_by_id(id).exec(conn).await?;
    Ok(result.rows_affected > 0)
}
