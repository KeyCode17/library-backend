//! Postgres/SeaORM `DeviceRepository` adapter.

use async_trait::async_trait;
use uuid::Uuid;

use persistence::entity::device;
use persistence::sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};

use crate::domain::{Device, DeviceRepository, NotificationError, Platform};

pub struct SeaOrmDeviceRepository {
    db: DatabaseConnection,
}

impl SeaOrmDeviceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn backend<E: std::fmt::Display>(error: E) -> NotificationError {
    NotificationError::Repository(error.to_string())
}

fn to_domain(model: device::Model) -> Result<Device, NotificationError> {
    let platform = Platform::parse(&model.platform).ok_or_else(|| {
        NotificationError::Repository(format!("unknown platform: {}", model.platform))
    })?;
    Ok(Device {
        id: model.id,
        user_id: model.user_id,
        token: model.token,
        platform,
        registered_at: model.registered_at,
    })
}

#[async_trait]
impl DeviceRepository for SeaOrmDeviceRepository {
    async fn upsert(&self, device: Device) -> Result<Device, NotificationError> {
        // Idempotent on (user, token): refresh an existing row, else insert.
        let existing = device::Entity::find()
            .filter(device::Column::UserId.eq(device.user_id))
            .filter(device::Column::Token.eq(device.token.clone()))
            .one(&self.db)
            .await
            .map_err(backend)?;

        let updated = match existing {
            Some(model) => {
                let mut active = model.into_active_model();
                active.platform = Set(device.platform.as_str().to_owned());
                active.registered_at = Set(device.registered_at);
                active.update(&self.db).await.map_err(backend)?
            }
            None => device::ActiveModel {
                id: Set(device.id),
                user_id: Set(device.user_id),
                token: Set(device.token),
                platform: Set(device.platform.as_str().to_owned()),
                registered_at: Set(device.registered_at),
            }
            .insert(&self.db)
            .await
            .map_err(backend)?,
        };
        to_domain(updated)
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Device>, NotificationError> {
        let models = device::Entity::find()
            .filter(device::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
            .map_err(backend)?;
        models.into_iter().map(to_domain).collect()
    }
}
