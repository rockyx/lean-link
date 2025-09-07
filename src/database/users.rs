use crate::database::entity::{prelude::TUsers, t_users};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

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
