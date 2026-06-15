//! `users` table schema. Source of truth for the migration DDL.
//!
//! Mirrors the IAM domain `User` (apps/iam). `password_hash` stores the Argon2
//! PHC string — never a plaintext password. `role` is the wire role string.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub email: String,
    pub password_hash: String,
    pub role: String,
    /// Whether the email has been verified (login is not blocked on this).
    pub verified: bool,
    /// Whether the account is active (deactivated accounts cannot log in).
    pub active: bool,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
