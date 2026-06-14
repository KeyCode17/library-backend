//! Shared persistence: SeaORM entities (the schema source of truth) and the
//! Postgres connection pool.
//!
//! `migration` derives DDL from these entities; each context's infrastructure
//! adapter queries through them. `sea_orm` is re-exported so every consumer uses
//! one version of the ORM.

pub mod db;
pub mod entity;

pub use sea_orm;
