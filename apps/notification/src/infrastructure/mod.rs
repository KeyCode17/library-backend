//! Infrastructure layer: in-memory stores, push senders (FCM + fake), config.

pub mod config;
pub mod fake_push_sender;
pub mod fcm_push_sender;
pub mod in_memory_devices;
pub mod in_memory_reminders;
pub mod system_clock;

pub use config::FcmConfig;
pub use fake_push_sender::FakePushSender;
pub use fcm_push_sender::FcmPushSender;
pub use in_memory_devices::InMemoryDeviceRepository;
pub use in_memory_reminders::InMemoryReminderRepository;
pub use system_clock::SystemClock;
