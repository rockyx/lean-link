use crate::database::entity::{PageResult, prelude::TSerialportConfigs, t_serialport_configs};
use sea_orm::{
    ActiveValue, DatabaseConnection, DbErr, EntityTrait, InsertResult,
    PaginatorTrait, QueryOrder,
};
use uuid::Uuid;

pub async fn insert_serialport_config(
    conn: &DatabaseConnection,
    config: t_serialport_configs::ActiveModel,
) -> Result<InsertResult<t_serialport_configs::ActiveModel>, DbErr> {
    TSerialportConfigs::insert(config).exec(conn).await
}

pub async fn find_serialport_config_by_id(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<t_serialport_configs::Model>, DbErr> {
    TSerialportConfigs::find_by_id(id).one(conn).await
}

pub async fn find_all_serialport_configs(
    conn: &DatabaseConnection,
) -> Result<Vec<t_serialport_configs::Model>, DbErr> {
    TSerialportConfigs::find()
        .all(conn)
        .await
}

pub async fn page_serialport_configs(
    conn: &DatabaseConnection,
    mut page_index: u64,
    mut page_size: u64,
) -> Result<PageResult<t_serialport_configs::Model>, DbErr> {
    if page_index == 0 {
        page_index = 1;
    }

    if page_size == 0 {
        page_size = 10;
    }

    let page_find = TSerialportConfigs::find()
        .order_by_asc(t_serialport_configs::Column::Id)
        .paginate(conn, page_size);

    let mut page = PageResult::<t_serialport_configs::Model> {
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

pub async fn update_serialport_config(
    conn: &DatabaseConnection,
    id: Uuid,
    config: t_serialport_configs::ActiveModel,
) -> Result<Option<t_serialport_configs::Model>, DbErr> {
    let existing = TSerialportConfigs::find_by_id(id).one(conn).await?;
    if existing.is_none() {
        return Ok(None);
    }

    let mut config = config;
    config.id = ActiveValue::set(id);
    
    TSerialportConfigs::update(config).exec(conn).await.map(Some)
}

pub async fn delete_serialport_config(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<bool, DbErr> {
    let result = TSerialportConfigs::delete_by_id(id).exec(conn).await?;
    Ok(result.rows_affected > 0)
}
