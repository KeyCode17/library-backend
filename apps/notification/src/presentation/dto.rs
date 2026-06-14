//! Wire DTOs. Mirrors the `DeviceRegistration` / `Notification` contract schemas.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{Device, Page, Platform, Reminder, ReminderKind};

/// `DeviceRegistration` request body.
#[derive(Debug, Deserialize)]
pub struct DeviceRegistrationRequest {
    pub token: String,
    pub platform: Platform,
}

/// `DeviceRegistration` response.
#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub platform: Platform,
    pub registered_at: DateTime<Utc>,
}

impl From<Device> for DeviceResponse {
    fn from(device: Device) -> Self {
        Self {
            id: device.id,
            user_id: device.user_id,
            token: device.token,
            platform: device.platform,
            registered_at: device.registered_at,
        }
    }
}

/// `Notification` — a logged reminder.
#[derive(Debug, Serialize)]
pub struct NotificationDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub loan_id: Uuid,
    pub kind: ReminderKind,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

impl From<Reminder> for NotificationDto {
    fn from(reminder: Reminder) -> Self {
        Self {
            id: reminder.id,
            user_id: reminder.user_id,
            loan_id: reminder.loan_id,
            kind: reminder.kind,
            message: reminder.message,
            created_at: reminder.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginationDto {
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

/// `NotificationList` envelope.
#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub data: Vec<NotificationDto>,
    pub pagination: PaginationDto,
}

impl From<Page<Reminder>> for NotificationListResponse {
    fn from(page: Page<Reminder>) -> Self {
        let pagination = PaginationDto {
            page: page.page,
            page_size: page.page_size,
            total: page.total,
            total_pages: page.total_pages(),
        };
        let data = page.items.into_iter().map(NotificationDto::from).collect();
        Self { data, pagination }
    }
}

/// Shared `Error { code, message }` body.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: &'static str,
}

impl ErrorBody {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}
