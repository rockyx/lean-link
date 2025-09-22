use crate::database::entity::{PageResult, prelude::TLogs, t_logs};
use chrono::Local;
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult, PaginatorTrait,
    QueryFilter, QueryOrder, UpdateResult, prelude::Json,
};
use uuid::Uuid;

pub async fn insert_log(
    conn: &DatabaseConnection,
    user_id: Uuid,
    action: String,
    details: Json,
) -> Result<InsertResult<t_logs::ActiveModel>, DbErr> {
    TLogs::insert(t_logs::ActiveModel {
        id: ActiveValue::set(Uuid::now_v7()),
        user_id: ActiveValue::set(Some(user_id)),
        action: ActiveValue::set(action),
        details: ActiveValue::set(details),
        created_at: ActiveValue::not_set(),
        deleted_at: ActiveValue::not_set(),
        updated_at: ActiveValue::not_set(),
    })
    .exec(conn)
    .await
}

pub async fn clear_all_logs(conn: &DatabaseConnection) -> Result<UpdateResult, DbErr> {
    TLogs::update_many()
        .set(t_logs::ActiveModel {
            deleted_at: ActiveValue::set(Some(Local::now().fixed_offset())),
            ..Default::default()
        })
        .filter(t_logs::Column::DeletedAt.is_null())
        .exec(conn)
        .await
}

pub async fn page_logs(
    conn: &DatabaseConnection,
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
        .paginate(conn, page_size);

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
