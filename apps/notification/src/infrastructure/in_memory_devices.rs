//! In-memory `DeviceRepository`. Stand-in until the Postgres adapter is wired
//! (the devices table schema lives in the `migration` crate).

use std::sync::RwLock;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{Device, DeviceRepository, NotificationError};

pub struct InMemoryDeviceRepository {
    devices: RwLock<Vec<Device>>,
}

impl InMemoryDeviceRepository {
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryDeviceRepository {
    fn default() -> Self {
        Self::new()
    }
}

fn poisoned() -> NotificationError {
    NotificationError::Repository("device store lock poisoned".to_owned())
}

#[async_trait]
impl DeviceRepository for InMemoryDeviceRepository {
    async fn upsert(&self, device: Device) -> Result<Device, NotificationError> {
        let mut guard = self.devices.write().map_err(|_| poisoned())?;
        match guard
            .iter_mut()
            .find(|existing| existing.user_id == device.user_id && existing.token == device.token)
        {
            Some(existing) => {
                existing.platform = device.platform;
                existing.registered_at = device.registered_at;
                Ok(existing.clone())
            }
            None => {
                guard.push(device.clone());
                Ok(device)
            }
        }
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Device>, NotificationError> {
        let found = {
            let guard = self.devices.read().map_err(|_| poisoned())?;
            guard
                .iter()
                .filter(|device| device.user_id == user_id)
                .cloned()
                .collect()
        };
        Ok(found)
    }
}
