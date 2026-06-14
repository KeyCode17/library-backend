//! The background scheduler loop (ADR 0006): periodically run the due-date scan.
//!
//! Thin on purpose — all the logic (and the tests) live in `RunReminderScan`.
//! The gateway constructs this and spawns `run()` on the Tokio runtime.

use std::sync::Arc;
use std::time::Duration;

use crate::domain::Clock;

use super::run_reminder_scan::RunReminderScan;

pub struct NotificationScheduler {
    scan: Arc<RunReminderScan>,
    clock: Arc<dyn Clock>,
    period: Duration,
}

impl NotificationScheduler {
    pub fn new(scan: Arc<RunReminderScan>, clock: Arc<dyn Clock>, period: Duration) -> Self {
        Self {
            scan,
            clock,
            period,
        }
    }

    /// Run forever: tick every `period`, scanning at the current time. A failing
    /// scan is logged and the loop continues.
    pub async fn run(self) {
        let mut ticker = tokio::time::interval(self.period);
        loop {
            ticker.tick().await;
            if let Err(error) = self.scan.execute(self.clock.now()).await {
                eprintln!("WARN [notification]: reminder scan failed: {error}");
            }
        }
    }
}
