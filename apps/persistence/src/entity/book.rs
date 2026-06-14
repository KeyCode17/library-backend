//! `books` table schema. Source of truth for the migration DDL.
//!
//! Mirrors the domain `Book` (apps/catalog) and the `Book` schema in
//! `contract/openapi.yaml`. Change a column here and regenerate the migration.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "books")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub title: String,
    pub author: String,
    pub isbn: String,
    pub shelf: String,
    pub row: i32,
    pub available: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
