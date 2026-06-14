//! Postgres/SeaORM `UserRepository` adapter.

use async_trait::async_trait;
use uuid::Uuid;

use persistence::entity::user;
use persistence::sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel,
    QueryFilter, Set,
};

use crate::domain::{IamError, Role, User, UserRepository};

pub struct SeaOrmUserRepository {
    db: DatabaseConnection,
}

impl SeaOrmUserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn to_domain(model: user::Model) -> Result<User, IamError> {
    let role = Role::parse(&model.role)
        .ok_or_else(|| IamError::Repository(format!("unknown stored role: {}", model.role)))?;
    Ok(User {
        id: model.id,
        email: model.email,
        password_hash: model.password_hash,
        role,
    })
}

fn backend<E: std::fmt::Display>(error: E) -> IamError {
    IamError::Repository(error.to_string())
}

/// Map an insert failure to `EmailAlreadyExists` on a unique-constraint violation
/// (the email unique index is the authoritative guard against duplicates).
fn map_insert_error(error: DbErr) -> IamError {
    let text = error.to_string().to_lowercase();
    if text.contains("unique") || text.contains("duplicate") || text.contains("23505") {
        IamError::EmailAlreadyExists
    } else {
        IamError::Repository(error.to_string())
    }
}

#[async_trait]
impl UserRepository for SeaOrmUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, IamError> {
        let model = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(backend)?;
        model.map(to_domain).transpose()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, IamError> {
        let model = user::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(backend)?;
        model.map(to_domain).transpose()
    }

    async fn insert(&self, user: User) -> Result<(), IamError> {
        let active = user::ActiveModel {
            id: Set(user.id),
            email: Set(user.email),
            password_hash: Set(user.password_hash),
            role: Set(user.role.as_str().to_owned()),
        };
        active.insert(&self.db).await.map_err(map_insert_error)?;
        Ok(())
    }

    async fn set_role(&self, id: Uuid, role: Role) -> Result<Option<User>, IamError> {
        let Some(model) = user::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(backend)?
        else {
            return Ok(None);
        };

        let mut active = model.into_active_model();
        active.role = Set(role.as_str().to_owned());
        let updated = active.update(&self.db).await.map_err(backend)?;
        to_domain(updated).map(Some)
    }
}
