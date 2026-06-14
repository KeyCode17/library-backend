//! Postgres/SeaORM `BookRepository` adapter.

use async_trait::async_trait;
use uuid::Uuid;

use persistence::entity::book;
use persistence::sea_orm::sea_query::Expr;
use persistence::sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::domain::{
    Book, BookFilter, BookRepository, ClaimOutcome, Page, PageRequest, RepositoryError,
};

pub struct SeaOrmBookRepository {
    db: DatabaseConnection,
}

impl SeaOrmBookRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn to_domain(model: book::Model) -> Book {
    Book {
        id: model.id,
        title: model.title,
        author: model.author,
        isbn: model.isbn,
        shelf: model.shelf,
        row: model.row,
        available: model.available,
    }
}

fn backend<E: std::fmt::Display>(error: E) -> RepositoryError {
    RepositoryError::Backend(error.to_string())
}

#[async_trait]
impl BookRepository for SeaOrmBookRepository {
    async fn list(
        &self,
        filter: &BookFilter,
        request: PageRequest,
    ) -> Result<Page<Book>, RepositoryError> {
        let mut query = book::Entity::find();
        if let Some(shelf) = &filter.shelf {
            query = query.filter(book::Column::Shelf.eq(shelf.clone()));
        }
        if let Some(row) = filter.row {
            query = query.filter(book::Column::Row.eq(row));
        }
        if let Some(isbn) = &filter.isbn {
            query = query.filter(book::Column::Isbn.eq(isbn.clone()));
        }

        let paginator = query
            .order_by_asc(book::Column::Id)
            .paginate(&self.db, u64::from(request.page_size()));
        let total = paginator.num_items().await.map_err(backend)?;
        let models = paginator
            .fetch_page(u64::from(request.page() - 1))
            .await
            .map_err(backend)?;

        Ok(Page {
            items: models.into_iter().map(to_domain).collect(),
            page: request.page(),
            page_size: request.page_size(),
            total,
        })
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Book>, RepositoryError> {
        let model = book::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(backend)?;
        Ok(model.map(to_domain))
    }

    async fn set_availability(
        &self,
        id: Uuid,
        available: bool,
    ) -> Result<Option<Book>, RepositoryError> {
        let Some(model) = book::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(backend)?
        else {
            return Ok(None);
        };

        let mut active = model.into_active_model();
        active.available = Set(available);
        let updated = active.update(&self.db).await.map_err(backend)?;
        Ok(Some(to_domain(updated)))
    }

    async fn claim_if_available(&self, id: Uuid) -> Result<ClaimOutcome, RepositoryError> {
        // Single atomic statement: UPDATE books SET available=false
        //                          WHERE id = ? AND available = true
        let result = book::Entity::update_many()
            .col_expr(book::Column::Available, Expr::value(false))
            .filter(book::Column::Id.eq(id))
            .filter(book::Column::Available.eq(true))
            .exec(&self.db)
            .await
            .map_err(backend)?;

        if result.rows_affected >= 1 {
            return Ok(ClaimOutcome::Claimed);
        }

        // Nothing transitioned: distinguish absent from already-unavailable.
        let exists = book::Entity::find_by_id(id)
            .count(&self.db)
            .await
            .map_err(backend)?
            > 0;
        Ok(if exists {
            ClaimOutcome::Unavailable
        } else {
            ClaimOutcome::NotFound
        })
    }
}
