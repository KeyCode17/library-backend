//! Postgres/SeaORM `ReminderRepository` adapter.

use async_trait::async_trait;
use uuid::Uuid;

use persistence::entity::reminder;
use persistence::sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::domain::{
    NotificationError, Page, PageRequest, Reminder, ReminderKind, ReminderRepository,
};

pub struct SeaOrmReminderRepository {
    db: DatabaseConnection,
}

impl SeaOrmReminderRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn backend<E: std::fmt::Display>(error: E) -> NotificationError {
    NotificationError::Repository(error.to_string())
}

fn to_domain(model: reminder::Model) -> Result<Reminder, NotificationError> {
    let kind = ReminderKind::parse(&model.kind).ok_or_else(|| {
        NotificationError::Repository(format!("unknown reminder kind: {}", model.kind))
    })?;
    Ok(Reminder {
        id: model.id,
        user_id: model.user_id,
        loan_id: model.loan_id,
        kind,
        message: model.message,
        created_at: model.created_at,
    })
}

#[async_trait]
impl ReminderRepository for SeaOrmReminderRepository {
    async fn insert(&self, reminder: Reminder) -> Result<(), NotificationError> {
        let active = reminder::ActiveModel {
            id: Set(reminder.id),
            user_id: Set(reminder.user_id),
            loan_id: Set(reminder.loan_id),
            kind: Set(reminder.kind.as_str().to_owned()),
            message: Set(reminder.message),
            created_at: Set(reminder.created_at),
        };
        active.insert(&self.db).await.map_err(backend)?;
        Ok(())
    }

    async fn exists(&self, loan_id: Uuid, kind: ReminderKind) -> Result<bool, NotificationError> {
        let count = reminder::Entity::find()
            .filter(reminder::Column::LoanId.eq(loan_id))
            .filter(reminder::Column::Kind.eq(kind.as_str()))
            .count(&self.db)
            .await
            .map_err(backend)?;
        Ok(count > 0)
    }

    async fn list_by_user(
        &self,
        user_id: Uuid,
        request: PageRequest,
    ) -> Result<Page<Reminder>, NotificationError> {
        let paginator = reminder::Entity::find()
            .filter(reminder::Column::UserId.eq(user_id))
            .order_by_asc(reminder::Column::CreatedAt)
            .order_by_asc(reminder::Column::Id)
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
}
