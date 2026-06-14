use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::domain::{
    DeviceRepository, LoanSource, NotificationError, PushNotification, PushSender, Reminder,
    ReminderKind, ReminderRepository,
};

/// How far ahead of the due date a "due soon" reminder fires.
fn due_soon_window() -> Duration {
    Duration::days(2)
}

/// Classify a loan by its due date relative to `now`.
fn classify(due_at: DateTime<Utc>, now: DateTime<Utc>) -> Option<ReminderKind> {
    if due_at <= now {
        Some(ReminderKind::Overdue)
    } else if due_at - now <= due_soon_window() {
        Some(ReminderKind::DueSoon)
    } else {
        None
    }
}

/// The testable scheduler step: scan active loans for due-soon/overdue ones and
/// push reminders. Idempotent — a reminder is created (and pushed) at most once
/// per (loan, kind), so repeated ticks don't spam.
pub struct RunReminderScan {
    loans: Arc<dyn LoanSource>,
    reminders: Arc<dyn ReminderRepository>,
    devices: Arc<dyn DeviceRepository>,
    push: Arc<dyn PushSender>,
}

impl RunReminderScan {
    pub fn new(
        loans: Arc<dyn LoanSource>,
        reminders: Arc<dyn ReminderRepository>,
        devices: Arc<dyn DeviceRepository>,
        push: Arc<dyn PushSender>,
    ) -> Self {
        Self {
            loans,
            reminders,
            devices,
            push,
        }
    }

    /// Run one scan at time `now`, returning the reminders newly created.
    pub async fn execute(&self, now: DateTime<Utc>) -> Result<Vec<Reminder>, NotificationError> {
        let active = self.loans.active_loans().await?;
        let mut created = Vec::new();

        for loan in active {
            let Some(kind) = classify(loan.due_at, now) else {
                continue;
            };
            if self.reminders.exists(loan.loan_id, kind).await? {
                continue; // already reminded for this (loan, kind)
            }

            let reminder = Reminder {
                id: uuid::Uuid::new_v4(),
                user_id: loan.user_id,
                loan_id: loan.loan_id,
                kind,
                message: kind.message().to_owned(),
                created_at: now,
            };
            self.reminders.insert(reminder.clone()).await?;

            let notification = PushNotification {
                title: kind.title().to_owned(),
                body: reminder.message.clone(),
            };
            for device in self.devices.list_by_user(loan.user_id).await? {
                // A failed push must not abort the scan; the sender logs it.
                let _ = self.push.send(&device.token, &notification).await;
            }

            created.push(reminder);
        }

        Ok(created)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Device, DueLoan, Platform};
    use crate::infrastructure::{
        FakePushSender, InMemoryDeviceRepository, InMemoryReminderRepository,
    };
    use async_trait::async_trait;
    use uuid::Uuid;

    struct FakeLoanSource {
        loans: Vec<DueLoan>,
    }

    #[async_trait]
    impl LoanSource for FakeLoanSource {
        async fn active_loans(&self) -> Result<Vec<DueLoan>, NotificationError> {
            Ok(self.loans.clone())
        }
    }

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).expect("timestamp")
    }

    struct Harness {
        scan: RunReminderScan,
        reminders: Arc<InMemoryReminderRepository>,
        push: Arc<FakePushSender>,
    }

    async fn harness(loans: Vec<DueLoan>, devices_for: &[(Uuid, &str)]) -> Harness {
        let reminders = Arc::new(InMemoryReminderRepository::new());
        let devices = Arc::new(InMemoryDeviceRepository::new());
        let push = Arc::new(FakePushSender::new());

        for (user_id, token) in devices_for {
            devices
                .upsert(Device {
                    id: Uuid::new_v4(),
                    user_id: *user_id,
                    token: (*token).to_owned(),
                    platform: Platform::Android,
                    registered_at: at(0),
                })
                .await
                .expect("seed device");
        }

        let scan = RunReminderScan::new(
            Arc::new(FakeLoanSource { loans }),
            reminders.clone(),
            devices,
            push.clone(),
        );
        Harness {
            scan,
            reminders,
            push,
        }
    }

    #[tokio::test]
    async fn due_soon_loan_produces_a_reminder_and_pushes() {
        let user = Uuid::new_v4();
        let now = at(1_000_000);
        // Due in one day — inside the two-day window.
        let loan = DueLoan {
            loan_id: Uuid::new_v4(),
            user_id: user,
            due_at: now + Duration::days(1),
        };
        let h = harness(vec![loan], &[(user, "device-token-1")]).await;

        let created = h.scan.execute(now).await.expect("scan");
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].kind, ReminderKind::DueSoon);

        let sent = h.push.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "device-token-1");
        assert_eq!(sent[0].1.title, "Loan due soon");
    }

    #[tokio::test]
    async fn overdue_loan_produces_an_overdue_reminder() {
        let user = Uuid::new_v4();
        let now = at(2_000_000);
        let loan = DueLoan {
            loan_id: Uuid::new_v4(),
            user_id: user,
            due_at: now - Duration::hours(1), // past due
        };
        let h = harness(vec![loan], &[(user, "tok")]).await;

        let created = h.scan.execute(now).await.expect("scan");
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].kind, ReminderKind::Overdue);
        assert_eq!(h.push.sent().len(), 1);
    }

    #[tokio::test]
    async fn loan_due_far_out_produces_nothing() {
        let user = Uuid::new_v4();
        let now = at(3_000_000);
        let loan = DueLoan {
            loan_id: Uuid::new_v4(),
            user_id: user,
            due_at: now + Duration::days(10),
        };
        let h = harness(vec![loan], &[(user, "tok")]).await;

        assert!(h.scan.execute(now).await.expect("scan").is_empty());
        assert_eq!(h.push.sent().len(), 0);
    }

    #[tokio::test]
    async fn scan_is_idempotent_across_ticks() {
        let user = Uuid::new_v4();
        let now = at(4_000_000);
        let loan = DueLoan {
            loan_id: Uuid::new_v4(),
            user_id: user,
            due_at: now + Duration::days(1),
        };
        let h = harness(vec![loan], &[(user, "tok")]).await;

        assert_eq!(h.scan.execute(now).await.expect("first").len(), 1);
        // A second tick a minute later creates nothing new and pushes nothing more.
        assert_eq!(
            h.scan
                .execute(now + Duration::minutes(1))
                .await
                .expect("second")
                .len(),
            0
        );
        assert_eq!(h.push.sent().len(), 1);
    }

    #[tokio::test]
    async fn reminder_is_logged_even_without_a_device_but_nothing_is_pushed() {
        let user = Uuid::new_v4();
        let now = at(5_000_000);
        let loan = DueLoan {
            loan_id: Uuid::new_v4(),
            user_id: user,
            due_at: now + Duration::days(1),
        };
        let h = harness(vec![loan], &[]).await; // no devices

        let created = h.scan.execute(now).await.expect("scan");
        assert_eq!(created.len(), 1);
        assert_eq!(h.push.sent().len(), 0);
        // It is still in the user's history.
        let page = h
            .reminders
            .list_by_user(user, crate::domain::PageRequest::new(1, 50))
            .await
            .expect("history");
        assert_eq!(page.total, 1);
    }
}
