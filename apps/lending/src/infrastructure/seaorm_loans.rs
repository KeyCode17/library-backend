//! Postgres/SeaORM `LoanRepository` adapter.

use async_trait::async_trait;
use uuid::Uuid;

use persistence::entity::loan;
use persistence::sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::domain::{LendingError, Loan, LoanRepository, LoanStatus, Page, PageRequest};

pub struct SeaOrmLoanRepository {
    db: DatabaseConnection,
}

impl SeaOrmLoanRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn backend<E: std::fmt::Display>(error: E) -> LendingError {
    LendingError::Repository(error.to_string())
}

fn to_domain(model: loan::Model) -> Result<Loan, LendingError> {
    let status = LoanStatus::parse(&model.status).ok_or_else(|| {
        LendingError::Repository(format!("unknown loan status: {}", model.status))
    })?;
    Ok(Loan {
        id: model.id,
        book_id: model.book_id,
        user_id: model.user_id,
        status,
        borrowed_at: model.borrowed_at,
        due_at: model.due_at,
        returned_at: model.returned_at,
        approved_by: model.approved_by,
        approved_at: model.approved_at,
    })
}

fn to_active(loan: Loan) -> loan::ActiveModel {
    loan::ActiveModel {
        id: Set(loan.id),
        book_id: Set(loan.book_id),
        user_id: Set(loan.user_id),
        status: Set(loan.status.as_str().to_owned()),
        borrowed_at: Set(loan.borrowed_at),
        due_at: Set(loan.due_at),
        returned_at: Set(loan.returned_at),
        approved_by: Set(loan.approved_by),
        approved_at: Set(loan.approved_at),
    }
}

async fn page_of(
    query: persistence::sea_orm::Select<loan::Entity>,
    db: &DatabaseConnection,
    request: PageRequest,
) -> Result<Page<Loan>, LendingError> {
    let paginator = query
        .order_by_asc(loan::Column::BorrowedAt)
        .order_by_asc(loan::Column::Id)
        .paginate(db, u64::from(request.page_size()));
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

#[async_trait]
impl LoanRepository for SeaOrmLoanRepository {
    async fn insert(&self, loan: Loan) -> Result<(), LendingError> {
        to_active(loan).insert(&self.db).await.map_err(backend)?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Loan>, LendingError> {
        let model = loan::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(backend)?;
        model.map(to_domain).transpose()
    }

    async fn update(&self, loan: Loan) -> Result<(), LendingError> {
        let exists = loan::Entity::find_by_id(loan.id)
            .count(&self.db)
            .await
            .map_err(backend)?
            > 0;
        if !exists {
            return Err(LendingError::LoanNotFound);
        }
        to_active(loan).update(&self.db).await.map_err(backend)?;
        Ok(())
    }

    async fn list_all(&self, request: PageRequest) -> Result<Page<Loan>, LendingError> {
        page_of(loan::Entity::find(), &self.db, request).await
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        request: PageRequest,
    ) -> Result<Page<Loan>, LendingError> {
        page_of(
            loan::Entity::find().filter(loan::Column::UserId.eq(user_id)),
            &self.db,
            request,
        )
        .await
    }

    async fn list_active(&self) -> Result<Vec<Loan>, LendingError> {
        let models = loan::Entity::find()
            .filter(loan::Column::Status.eq(LoanStatus::Borrowed.as_str()))
            .all(&self.db)
            .await
            .map_err(backend)?;
        models.into_iter().map(to_domain).collect()
    }
}
