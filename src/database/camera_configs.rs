use crate::database::entity::{PageResult, prelude::TCameraConfigs, t_camera_configs};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

pub async fn insert_camera_config(
    conn: &DatabaseConnection,
    config: t_camera_configs::ActiveModel,
) -> Result<InsertResult<t_camera_configs::ActiveModel>, DbErr> {
    TCameraConfigs::insert(config).exec(conn).await
}

pub async fn find_camera_config_by_id(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<t_camera_configs::Model>, DbErr> {
    TCameraConfigs::find_by_id(id).one(conn).await
}

pub async fn find_all_camera_configs(
    conn: &DatabaseConnection,
) -> Result<Vec<t_camera_configs::Model>, DbErr> {
    TCameraConfigs::find().all(conn).await
}

pub async fn find_camera_configs_by_enabled(
    conn: &DatabaseConnection,
    enabled: bool,
) -> Result<Vec<t_camera_configs::Model>, DbErr> {
    TCameraConfigs::find()
        .filter(t_camera_configs::Column::Enabled.eq(enabled))
        .all(conn)
        .await
}

pub async fn find_camera_config_by_key(
    conn: &DatabaseConnection,
    key: &str,
) -> Result<Option<t_camera_configs::Model>, DbErr> {
    TCameraConfigs::find()
        .filter(t_camera_configs::Column::Key.eq(key))
        .one(conn)
        .await
}

pub async fn find_camera_config_by_serial_number(
    conn: &DatabaseConnection,
    serial_number: &str,
) -> Result<Option<t_camera_configs::Model>, DbErr> {
    TCameraConfigs::find()
        .filter(t_camera_configs::Column::SerialNumber.eq(serial_number))
        .one(conn)
        .await
}

pub async fn page_camera_configs(
    conn: &DatabaseConnection,
    mut page_index: u64,
    mut page_size: u64,
) -> Result<PageResult<t_camera_configs::Model>, DbErr> {
    if page_index == 0 {
        page_index = 1;
    }

    if page_size == 0 {
        page_size = 10;
    }

    let page_find = TCameraConfigs::find()
        .order_by_asc(t_camera_configs::Column::Id)
        .paginate(conn, page_size);

    let mut page = PageResult::<t_camera_configs::Model> {
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

pub async fn update_camera_config(
    conn: &DatabaseConnection,
    id: Uuid,
    config: t_camera_configs::ActiveModel,
) -> Result<Option<t_camera_configs::Model>, DbErr> {
    let existing = TCameraConfigs::find_by_id(id).one(conn).await?;
    if existing.is_none() {
        return Ok(None);
    }

    let mut config = config;
    config.id = ActiveValue::set(id);

    TCameraConfigs::update(config).exec(conn).await.map(Some)
}

pub async fn delete_camera_config(
    conn: &DatabaseConnection,
    id: Uuid,
) -> Result<bool, DbErr> {
    let result = TCameraConfigs::delete_by_id(id).exec(conn).await?;
    Ok(result.rows_affected > 0)
}
