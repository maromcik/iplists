use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub overall: Status,
    pub db: Status,
    pub locations: ComponentStatus,
    pub asns: ComponentStatus,
    pub geo: ComponentStatus,
    pub blocklist: ComponentStatus,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub status_code: i32,
    pub status_meaning: String,
    pub message: String,
}

impl Status {
    pub fn ok() -> Self {
        Self {
            status_code: 0,
            status_meaning: "ok".to_string(),
            message: "updated".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub status: Status,
    pub last_update: String,
    pub next_update: String,
}

impl UpdateStatus {
    pub fn ok(next_update: OffsetDateTime) -> Self {
        Self {
            status: Status::ok(),
            last_update: OffsetDateTime::now_utc().to_string(),
            next_update: next_update.to_string(),
        }
    }
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            status: Status::ok(),
            last_update: OffsetDateTime::now_utc().to_string(),
            next_update: OffsetDateTime::now_utc().to_string(),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub status: Status,
    pub update: UpdateStatus,
}

impl ComponentStatus {
    pub fn ok(next_update: OffsetDateTime) -> Self {
        Self {
            status: Status::ok(),
            update: UpdateStatus::ok(next_update),
        }
    }
}
