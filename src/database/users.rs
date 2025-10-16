use crate::database::entity::{prelude::TUsers, t_users};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub async fn find_user_by_name(
    conn: &DatabaseConnection,
    username: String,
) -> Result<Option<t_users::Model>, DbErr> {
    TUsers::find()
        .filter(
            t_users::Column::Username
                .eq(username)
                .and(t_users::Column::DeletedAt.is_null()),
        )
        .one(conn)
        .await
}

pub async fn find_user_by_id(
    conn: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Option<t_users::Model>, DbErr> {
    TUsers::find()
        .filter(
            t_users::Column::Id
                .eq(user_id)
                .and(t_users::Column::DeletedAt.is_null()),
        )
        .one(conn)
        .await
}
