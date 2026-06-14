//! Pagination value objects.
//!
//! Intentionally mirrors `catalog`'s pagination rather than depending on it (the
//! contexts stay decoupled). A future `shared-kernel` crate is the place to unify
//! these if the duplication grows.

pub const MAX_PAGE_SIZE: u32 = 100;
pub const DEFAULT_PAGE_SIZE: u32 = 20;

/// A validated, 1-based page request.
#[derive(Debug, Clone, Copy)]
pub struct PageRequest {
    page: u32,
    page_size: u32,
}

impl PageRequest {
    pub fn new(page: u32, page_size: u32) -> Self {
        Self {
            page: page.max(1),
            page_size: page_size.clamp(1, MAX_PAGE_SIZE),
        }
    }

    pub fn page(&self) -> u32 {
        self.page
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    pub fn offset(&self) -> u64 {
        u64::from(self.page - 1) * u64::from(self.page_size)
    }
}

/// A page of items plus the totals for pagination metadata.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

impl<T> Page<T> {
    pub fn total_pages(&self) -> u32 {
        if self.page_size == 0 {
            return 0;
        }
        self.total.div_ceil(u64::from(self.page_size)) as u32
    }
}
