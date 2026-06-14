//! Application layer: the lending use cases. Authorization (RBAC + loan
//! ownership) is enforced here, reusing the IAM principal/permission model.

pub mod approve_loan;
pub mod borrow_book;
pub mod list_loans;
pub mod return_loan;

pub use approve_loan::ApproveLoan;
pub use borrow_book::BorrowBook;
pub use list_loans::ListLoans;
pub use return_loan::ReturnLoan;
