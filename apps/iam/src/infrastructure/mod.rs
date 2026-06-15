//! Infrastructure layer: adapters for hashing, tokens, email, storage, config.

pub mod argon2_hasher;
pub mod config;
pub mod fake_email_sender;
pub mod in_memory_email_tokens;
pub mod in_memory_users;
pub mod jwt;
pub mod resend_email_sender;
pub mod seaorm_email_tokens;
pub mod seaorm_users;
pub mod system_clock;
pub mod token_generator;

pub use argon2_hasher::Argon2PasswordHasher;
pub use config::IamConfig;
pub use fake_email_sender::FakeEmailSender;
pub use in_memory_email_tokens::InMemoryEmailTokenRepository;
pub use in_memory_users::InMemoryUserRepository;
pub use jwt::JwtTokenService;
pub use resend_email_sender::ResendEmailSender;
pub use seaorm_email_tokens::SeaOrmEmailTokenRepository;
pub use seaorm_users::SeaOrmUserRepository;
pub use system_clock::SystemClock;
pub use token_generator::RandomTokenGenerator;
