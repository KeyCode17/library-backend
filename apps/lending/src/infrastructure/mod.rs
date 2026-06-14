//! Infrastructure layer: in-memory loan store and the system clock.

pub mod in_memory_loans;
pub mod seaorm_loans;
pub mod system_clock;

pub use in_memory_loans::InMemoryLoanRepository;
pub use seaorm_loans::SeaOrmLoanRepository;
pub use system_clock::SystemClock;
