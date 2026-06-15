//! `email_tokens` table schema: single-use, expiring tokens for email
//! verification and password reset. Only the SHA-256 hash of the token is stored.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "email_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    /// `verify_email` or `password_reset`.
    pub kind: String,
    /// Hex SHA-256 of the random token. Unique so lookup is by hash.
    #[sea_orm(unique)]
    pub token_hash: String,
    pub expires_at: DateTimeUtc,
    /// When the token was consumed (single-use); `None` while still valid.
    pub consumed_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
