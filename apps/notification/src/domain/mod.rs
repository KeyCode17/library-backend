//! Domain layer: entities, value objects, and ports. Pure.

pub mod device;
pub mod due_loan;
pub mod error;
pub mod pagination;
pub mod ports;
pub mod reminder;

pub use device::{Device, Platform};
pub use due_loan::DueLoan;
pub use error::NotificationError;
pub use pagination::{Page, PageRequest};
pub use ports::{
    Clock, DeviceRepository, LoanSource, PushNotification, PushSender, ReminderRepository,
};
pub use reminder::{Reminder, ReminderKind};
