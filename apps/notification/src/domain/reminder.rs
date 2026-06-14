//! A reminder produced by the due-date scan and logged for history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Why a reminder fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderKind {
    /// The loan is due within the due-soon window.
    DueSoon,
    /// The loan is past its due date.
    Overdue,
}

impl ReminderKind {
    /// Short push title for this kind.
    pub fn title(self) -> &'static str {
        match self {
            ReminderKind::DueSoon => "Loan due soon",
            ReminderKind::Overdue => "Loan overdue",
        }
    }

    /// Default reminder body.
    pub fn message(self) -> &'static str {
        match self {
            ReminderKind::DueSoon => "One of your loans is due soon. Please return it on time.",
            ReminderKind::Overdue => {
                "One of your loans is overdue. Please return it as soon as possible."
            }
        }
    }

    /// Stable wire string (matches the serde representation).
    pub fn as_str(self) -> &'static str {
        match self {
            ReminderKind::DueSoon => "due_soon",
            ReminderKind::Overdue => "overdue",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "due_soon" => Some(ReminderKind::DueSoon),
            "overdue" => Some(ReminderKind::Overdue),
            _ => None,
        }
    }
}

/// A logged reminder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub loan_id: Uuid,
    pub kind: ReminderKind,
    pub message: String,
    pub created_at: DateTime<Utc>,
}
