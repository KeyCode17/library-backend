//! Ports the notification use cases depend on.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::device::Device;
use super::due_loan::DueLoan;
use super::error::NotificationError;
use super::pagination::{Page, PageRequest};
use super::reminder::{Reminder, ReminderKind};

/// Persistence port for registered devices.
#[async_trait]
pub trait DeviceRepository: Send + Sync {
    /// Register (or refresh) a device token. Idempotent on (user, token).
    async fn upsert(&self, device: Device) -> Result<Device, NotificationError>;
    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Device>, NotificationError>;
}

/// Persistence port for the reminder log.
#[async_trait]
pub trait ReminderRepository: Send + Sync {
    async fn insert(&self, reminder: Reminder) -> Result<(), NotificationError>;
    /// Whether a reminder of this kind already exists for the loan — the scan uses
    /// it to stay idempotent across ticks (no duplicate pushes).
    async fn exists(&self, loan_id: Uuid, kind: ReminderKind) -> Result<bool, NotificationError>;
    async fn list_by_user(
        &self,
        user_id: Uuid,
        request: PageRequest,
    ) -> Result<Page<Reminder>, NotificationError>;
}

/// Read-only view of active loans, for due-date scanning. Abstract so the
/// notification domain does not depend on `lending`; the gateway bridges it.
#[async_trait]
pub trait LoanSource: Send + Sync {
    async fn active_loans(&self) -> Result<Vec<DueLoan>, NotificationError>;
}

/// A push payload.
#[derive(Debug, Clone)]
pub struct PushNotification {
    pub title: String,
    pub body: String,
}

/// Push transport port. Implemented by the FCM adapter (real, credential-gated)
/// and a fake (tests).
#[async_trait]
pub trait PushSender: Send + Sync {
    async fn send(
        &self,
        device_token: &str,
        notification: &PushNotification,
    ) -> Result<(), NotificationError>;
}

/// Clock port, so the scan is testable with an injected time.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
