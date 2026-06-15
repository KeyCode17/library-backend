//! Postgres/SeaORM `EmailTokenRepository` adapter.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use persistence::entity::email_token;
use persistence::sea_orm::sea_query::Expr;
use persistence::sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::domain::{EmailToken, EmailTokenKind, EmailTokenRepository, IamError};

pub struct SeaOrmEmailTokenRepository {
    db: DatabaseConnection,
}

impl SeaOrmEmailTokenRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn backend<E: std::fmt::Display>(error: E) -> IamError {
    IamError::Repository(error.to_string())
}

fn to_domain(model: email_token::Model) -> Result<EmailToken, IamError> {
    let kind = EmailTokenKind::parse(&model.kind)
        .ok_or_else(|| IamError::Repository(format!("unknown token kind: {}", model.kind)))?;
    Ok(EmailToken {
        id: model.id,
        user_id: model.user_id,
        kind,
        token_hash: model.token_hash,
        expires_at: model.expires_at,
        consumed_at: model.consumed_at,
        created_at: model.created_at,
    })
}

#[async_trait]
impl EmailTokenRepository for SeaOrmEmailTokenRepository {
    async fn insert(&self, token: EmailToken) -> Result<(), IamError> {
        let active = email_token::ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            kind: Set(token.kind.as_str().to_owned()),
            token_hash: Set(token.token_hash),
            expires_at: Set(token.expires_at),
            consumed_at: Set(token.consumed_at),
            created_at: Set(token.created_at),
        };
        active.insert(&self.db).await.map_err(backend)?;
        Ok(())
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<EmailToken>, IamError> {
        let model = email_token::Entity::find()
            .filter(email_token::Column::TokenHash.eq(token_hash))
            .one(&self.db)
            .await
            .map_err(backend)?;
        model.map(to_domain).transpose()
    }

    async fn consume(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool, IamError> {
        // Atomic single-use: only transition a not-yet-consumed token.
        let result = email_token::Entity::update_many()
            .col_expr(email_token::Column::ConsumedAt, Expr::value(at))
            .filter(email_token::Column::Id.eq(id))
            .filter(email_token::Column::ConsumedAt.is_null())
            .exec(&self.db)
            .await
            .map_err(backend)?;
        Ok(result.rows_affected >= 1)
    }

    async fn consume_all_for_user(
        &self,
        user_id: Uuid,
        kind: EmailTokenKind,
        at: DateTime<Utc>,
    ) -> Result<u64, IamError> {
        // UPDATE email_tokens SET consumed_at=$at
        //   WHERE user_id=$id AND kind=$kind AND consumed_at IS NULL
        let result = email_token::Entity::update_many()
            .col_expr(email_token::Column::ConsumedAt, Expr::value(at))
            .filter(email_token::Column::UserId.eq(user_id))
            .filter(email_token::Column::Kind.eq(kind.as_str()))
            .filter(email_token::Column::ConsumedAt.is_null())
            .exec(&self.db)
            .await
            .map_err(backend)?;
        Ok(result.rows_affected)
    }
}
