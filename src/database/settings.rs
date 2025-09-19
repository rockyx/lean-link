use crate::database::entity::{prelude::TSettings, t_settings};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
    sea_query::OnConflict,
};
use serde::{Serialize, de::DeserializeOwned};

pub async fn setting_get_x<T>(conn: &DatabaseConnection, key: &str) -> Result<T, DbErr>
where
    T: DeserializeOwned + Default,
{
    let model = TSettings::find()
        .filter(t_settings::Column::Key.eq(key))
        .one(conn)
        .await?;

    Ok(match model {
        None => T::default(),
        Some(config) => serde_json::from_value(config.value).unwrap_or_default(),
    })
}

pub async fn setting_set_x<T>(
    conn: &DatabaseConnection,
    key: &str,
    value: T,
) -> Result<(), DbErr>
where
    T: Serialize + Default,
{
    let json_value = serde_json::to_value(value).map_err(|e| DbErr::Json(e.to_string()))?;
    TSettings::insert(t_settings::ActiveModel {
        id: ActiveValue::not_set(),
        key: ActiveValue::set(key.into()),
        value: ActiveValue::set(json_value),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([t_settings::Column::Key])
            .update_column(t_settings::Column::Value)
            .to_owned(),
    )
    .exec(conn)
    .await?;

    Ok(())
}
