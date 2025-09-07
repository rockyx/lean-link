use crate::database::entity::{prelude::TSettings, t_settings};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
    sea_query::OnConflict,
};
use serde::{Serialize, de::DeserializeOwned};

pub async fn setting_get_x<T>(conn: &DatabaseConnection, name: &str) -> Result<T, DbErr>
where
    T: DeserializeOwned + Default,
{
    let model = TSettings::find()
        .filter(t_settings::Column::Name.eq(name))
        .filter(t_settings::Column::Sequence.eq(1))
        .one(conn)
        .await?;

    Ok(match model {
        None => T::default(),
        Some(config) => serde_json::from_value(config.value).unwrap_or_default(),
    })
}

pub async fn setting_set_x<T>(
    conn: &DatabaseConnection,
    name: &str,
    desc: &str,
    value: T,
) -> Result<(), DbErr>
where
    T: Serialize + Default,
{
    let json_value = serde_json::to_value(value).map_err(|e| DbErr::Json(e.to_string()))?;
    TSettings::insert(t_settings::ActiveModel {
        id: ActiveValue::not_set(),
        name: ActiveValue::set(name.into()),
        sequence: ActiveValue::set(1.into()),
        description: ActiveValue::set(Some(desc.into())),
        value: ActiveValue::set(json_value),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([t_settings::Column::Name, t_settings::Column::Sequence])
            .update_column(t_settings::Column::Value)
            .to_owned(),
    )
    .exec(conn)
    .await?;

    Ok(())
}
