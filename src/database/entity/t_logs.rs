use sea_orm::{Set, entity::prelude::*};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "t_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub user_id: i64,
    pub action: String,
    pub details: Json,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub updated_at: chrono::DateTime<chrono::Local>,
    pub deleted_at: Option<chrono::DateTime<chrono::Local>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            created_at: Set(chrono::Local::now()),
            updated_at: Set(chrono::Local::now()),
            deleted_at: Set(None),
            ..ActiveModelTrait::default()
        }
    }

    fn before_save<'life0, 'async_trait, C>(
        mut self,
        _db: &'life0 C,
        insert: bool,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, DbErr>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        C: ConnectionTrait,
        C: 'async_trait,
        'life0: 'async_trait,
        Self: ::core::marker::Send + 'async_trait,
    {
        Box::pin(async move {
            if insert {
                self.created_at = Set(chrono::Local::now());
            }
            self.updated_at = Set(chrono::Local::now());
            Ok(self)
        })
    }
}
