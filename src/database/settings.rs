use crate::database::entity::{prelude::TSettings, t_settings};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;

pub async fn setting_get_x<T>(conn: &DatabaseConnection, key: &str) -> Result<T, DbErr>
where
    T: DeserializeOwned + Default,
{
    let model = TSettings::find()
        .filter(
            t_settings::Column::Key
                .eq(key)
                .and(t_settings::Column::DeletedAt.is_null()),
        )
        .one(conn)
        .await?;

    Ok(match model {
        None => T::default(),
        Some(config) => serde_json::from_value(config.value).unwrap_or_default(),
    })
}

pub async fn setting_set_x<T>(conn: &DatabaseConnection, key: &str, value: T) -> Result<(), DbErr>
where
    T: Serialize + Default,
{
    let setting_model = TSettings::find()
        .filter(
            t_settings::Column::Key
                .eq(key)
                .and(t_settings::Column::DeletedAt.is_null()),
        )
        .one(conn)
        .await?;

    let json_value = serde_json::to_value(value).map_err(|e| DbErr::Json(e.to_string()))?;
    let mut model = t_settings::ActiveModel {
        key: ActiveValue::set(key.into()),
        value: ActiveValue::set(json_value),
        ..Default::default()
    };

    if setting_model.is_none() {
        model.id = ActiveValue::set(Uuid::now_v7());
        TSettings::insert(model).exec(conn).await?;
    } else {
        model.id = ActiveValue::set(setting_model.unwrap().id);
        TSettings::update(model).exec(conn).await?;
    };

    Ok(())
}
