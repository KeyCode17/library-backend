//! Domain layer: the loan entity, value objects, and ports. Pure.

pub mod error;
pub mod loan;
pub mod pagination;
pub mod ports;
pub mod status;

pub use error::LendingError;
pub use loan::Loan;
pub use pagination::{Page, PageRequest};
pub use ports::{BookGateway, ClaimOutcome, Clock, LoanRepository};
pub use status::LoanStatus;
