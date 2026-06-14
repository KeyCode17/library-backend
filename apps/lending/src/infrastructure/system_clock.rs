//! Wall-clock adapter for the `Clock` port.

use chrono::{DateTime, Utc};

use crate::domain::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
