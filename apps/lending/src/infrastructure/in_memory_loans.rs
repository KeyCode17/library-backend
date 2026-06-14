//! In-memory `LoanRepository`. Stand-in until the Postgres/SeaORM adapter is
//! wired (the loans table schema lives in the `migration` crate).

use std::sync::RwLock;

use uuid::Uuid;

use async_trait::async_trait;

use crate::domain::{LendingError, Loan, LoanRepository, LoanStatus, Page, PageRequest};

pub struct InMemoryLoanRepository {
    loans: RwLock<Vec<Loan>>,
}

impl InMemoryLoanRepository {
    pub fn new() -> Self {
        Self {
            loans: RwLock::new(Vec::new()),
        }
    }

    pub fn seeded_with(initial: Vec<Loan>) -> Self {
        Self {
            loans: RwLock::new(initial),
        }
    }
}

impl Default for InMemoryLoanRepository {
    fn default() -> Self {
        Self::new()
    }
}

fn poisoned() -> LendingError {
    LendingError::Repository("loan store lock poisoned".to_owned())
}

/// Paginate a pre-collected, insertion-ordered slice of loans.
fn paginate(all: Vec<Loan>, request: PageRequest) -> Page<Loan> {
    let total = all.len() as u64;
    let offset = request.offset() as usize;
    let items = all
        .into_iter()
        .skip(offset)
        .take(request.page_size() as usize)
        .collect();
    Page {
        items,
        page: request.page(),
        page_size: request.page_size(),
        total,
    }
}

#[async_trait]
impl LoanRepository for InMemoryLoanRepository {
    async fn insert(&self, loan: Loan) -> Result<(), LendingError> {
        let mut guard = self.loans.write().map_err(|_| poisoned())?;
        guard.push(loan);
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Loan>, LendingError> {
        let found = {
            let guard = self.loans.read().map_err(|_| poisoned())?;
            guard.iter().find(|loan| loan.id == id).cloned()
        };
        Ok(found)
    }

    async fn update(&self, loan: Loan) -> Result<(), LendingError> {
        let mut guard = self.loans.write().map_err(|_| poisoned())?;
        match guard.iter_mut().find(|existing| existing.id == loan.id) {
            Some(slot) => {
                *slot = loan;
                Ok(())
            }
            None => Err(LendingError::LoanNotFound),
        }
    }

    async fn list_all(&self, request: PageRequest) -> Result<Page<Loan>, LendingError> {
        let all = {
            let guard = self.loans.read().map_err(|_| poisoned())?;
            guard.clone()
        };
        Ok(paginate(all, request))
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        request: PageRequest,
    ) -> Result<Page<Loan>, LendingError> {
        let mine = {
            let guard = self.loans.read().map_err(|_| poisoned())?;
            guard
                .iter()
                .filter(|loan| loan.user_id == user_id)
                .cloned()
                .collect()
        };
        Ok(paginate(mine, request))
    }

    async fn list_active(&self) -> Result<Vec<Loan>, LendingError> {
        let active = {
            let guard = self.loans.read().map_err(|_| poisoned())?;
            guard
                .iter()
                .filter(|loan| loan.status == LoanStatus::Borrowed)
                .cloned()
                .collect()
        };
        Ok(active)
    }
}
