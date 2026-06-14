//! Notification bounded context: due-date reminders pushed via FCM (ADR 0006).
//!
//! Delivery is a background scheduler (tokio interval) + FCM push, not the REST
//! template. Persistence (device registry + reminder log) follows the skill,
//! in-memory for now.
//!
//! Hexagonal layering (ADR 0002):
//! - `domain` — `Device`, `Reminder`, `DueLoan`, the `DeviceRepository` /
//!   `ReminderRepository` / `LoanSource` / `PushSender` / `Clock` ports, errors.
//! - `application` — register-device, list-notifications, and `RunReminderScan`
//!   (the testable scheduler step), plus the scheduler loop.
//! - `infrastructure` — in-memory stores, the FCM and fake push senders, config.
//! - `presentation` — device registration + notification history (both bearer).

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
