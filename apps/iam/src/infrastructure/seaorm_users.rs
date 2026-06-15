//! Postgres/SeaORM `UserRepository` adapter.

use async_trait::async_trait;
use uuid::Uuid;

use persistence::entity::user;
use persistence::sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::domain::{IamError, Page, PageRequest, Role, User, UserRepository};

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
        verified: model.verified,
        active: model.active,
        created_at: model.created_at,
    })
}

fn backend<E: std::fmt::Display>(error: E) -> IamError {
    IamError::Repository(error.to_string())
}

fn is_unique_violation(error: &DbErr) -> bool {
    let text = error.to_string().to_lowercase();
    text.contains("unique") || text.contains("duplicate") || text.contains("23505")
}

fn map_email_conflict(error: DbErr) -> IamError {
    if is_unique_violation(&error) {
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
            verified: Set(user.verified),
            active: Set(user.active),
            created_at: Set(user.created_at),
        };
        active.insert(&self.db).await.map_err(map_email_conflict)?;
        Ok(())
    }

    async fn set_role(&self, id: Uuid, role: Role) -> Result<Option<User>, IamError> {
        self.update_field(id, |active| active.role = Set(role.as_str().to_owned()))
            .await
    }

    async fn list(&self, request: PageRequest) -> Result<Page<User>, IamError> {
        let paginator = user::Entity::find()
            .order_by_asc(user::Column::CreatedAt)
            .order_by_asc(user::Column::Id)
            .paginate(&self.db, u64::from(request.page_size()));
        let total = paginator.num_items().await.map_err(backend)?;
        let models = paginator
            .fetch_page(u64::from(request.page() - 1))
            .await
            .map_err(backend)?;
        let items = models
            .into_iter()
            .map(to_domain)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page {
            items,
            page: request.page(),
            page_size: request.page_size(),
            total,
        })
    }

    async fn set_email(&self, id: Uuid, email: &str) -> Result<Option<User>, IamError> {
        let Some(model) = user::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(backend)?
        else {
            return Ok(None);
        };
        let mut active = model.into_active_model();
        active.email = Set(email.to_owned());
        let updated = active.update(&self.db).await.map_err(map_email_conflict)?;
        to_domain(updated).map(Some)
    }

    async fn set_active(&self, id: Uuid, active: bool) -> Result<Option<User>, IamError> {
        self.update_field(id, |model| model.active = Set(active))
            .await
    }

    async fn set_password_hash(&self, id: Uuid, hash: &str) -> Result<Option<User>, IamError> {
        self.update_field(id, |model| model.password_hash = Set(hash.to_owned()))
            .await
    }

    async fn set_verified(&self, id: Uuid, verified: bool) -> Result<Option<User>, IamError> {
        self.update_field(id, |model| model.verified = Set(verified))
            .await
    }

    async fn delete(&self, id: Uuid) -> Result<bool, IamError> {
        let result = user::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(backend)?;
        Ok(result.rows_affected > 0)
    }

    async fn count_active_admins(&self) -> Result<u64, IamError> {
        user::Entity::find()
            .filter(user::Column::Role.eq(Role::Admin.as_str()))
            .filter(user::Column::Active.eq(true))
            .count(&self.db)
            .await
            .map_err(backend)
    }
}

impl SeaOrmUserRepository {
    /// Load a user, apply `mutate` to its active model, and persist — returning the
    /// updated domain user, or `None` if absent.
    async fn update_field<F>(&self, id: Uuid, mutate: F) -> Result<Option<User>, IamError>
    where
        F: FnOnce(&mut user::ActiveModel),
    {
        let Some(model) = user::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(backend)?
        else {
            return Ok(None);
        };
        let mut active = model.into_active_model();
        mutate(&mut active);
        let updated = active.update(&self.db).await.map_err(backend)?;
        to_domain(updated).map(Some)
    }
}
