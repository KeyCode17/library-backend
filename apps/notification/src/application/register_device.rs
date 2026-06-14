use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{Clock, Device, DeviceRepository, NotificationError, Platform};

/// Use case: register (or refresh) an FCM device token for a user.
pub struct RegisterDevice {
    devices: Arc<dyn DeviceRepository>,
    clock: Arc<dyn Clock>,
}

impl RegisterDevice {
    pub fn new(devices: Arc<dyn DeviceRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { devices, clock }
    }

    pub async fn execute(
        &self,
        user_id: Uuid,
        token: String,
        platform: Platform,
    ) -> Result<Device, NotificationError> {
        let token = token.trim().to_owned();
        if token.is_empty() {
            return Err(NotificationError::InvalidToken);
        }

        let device = Device {
            id: Uuid::new_v4(),
            user_id,
            token,
            platform,
            registered_at: self.clock.now(),
        };
        self.devices.upsert(device).await
    }
}
