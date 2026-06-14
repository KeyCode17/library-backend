//! `loans` table schema. Source of truth for the migration DDL.
//!
//! Mirrors the lending domain `Loan`. `status` is the wire status string;
//! nullable columns map to the lifecycle fields that fill in over time.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "loans")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub book_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub borrowed_at: DateTimeUtc,
    pub due_at: DateTimeUtc,
    pub returned_at: Option<DateTimeUtc>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
