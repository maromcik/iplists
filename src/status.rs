use croner::Cron;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::{OffsetDateTime, UtcOffset};

use crate::error::AppError;

/// Current time in the system's local timezone; falls back to UTC if the
/// local offset cannot be determined.
fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusCode {
    #[default]
    Ok = 0,
    Warning = 1,
    Error = 2,
    Disaster = 3,
}

impl StatusCode {
    pub fn meaning(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Disaster => "disaster",
        }
    }
}

impl Serialize for StatusCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(*self as i32)
    }
}

impl<'de> Deserialize<'de> for StatusCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match i32::deserialize(deserializer)? {
            0 => Ok(Self::Ok),
            1 => Ok(Self::Warning),
            2 => Ok(Self::Error),
            3 => Ok(Self::Disaster),
            other => Err(serde::de::Error::custom(format!(
                "unknown status code: {other}"
            ))),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub status_code: StatusCode,
    pub status_meaning: String,
    pub message: String,
}

impl Status {
    pub fn new(status_code: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status_code,
            status_meaning: status_code.meaning().to_string(),
            message: message.into(),
        }
    }

    pub fn ok(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Ok, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Error, message)
    }

    pub fn never_updated() -> Self {
        Self {
            status_code: StatusCode::Warning,
            status_meaning: "never updated".to_string(),
            message: "list has never been updated".to_string(),
        }
    }

    pub fn disaster(message: impl Into<String>) -> Self {
        Self::new(StatusCode::Disaster, message)
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_update: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_update: Option<OffsetDateTime>,
}

impl UpdateStatus {
    pub fn update_new(next_update: Option<OffsetDateTime>) -> Self {
        Self {
            last_update: Some(now_local()),
            next_update,
        }
    }

    pub fn update(&mut self, next_update: Option<OffsetDateTime>) {
        self.last_update = Some(now_local());
        self.next_update = next_update;
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub status: Status,
    pub update: UpdateStatus,
}

impl ComponentStatus {
    /// Component is healthy and has just been refreshed.
    pub fn ok_new(next_update: Option<OffsetDateTime>) -> Self {
        Self {
            status: Status::ok("Component is up-to-date"),
            update: UpdateStatus::update_new(next_update),
        }
    }

    pub fn ok(&mut self, message: impl Into<String>) {
        self.status = Status::ok(message);
    }
    pub fn warning(&mut self, message: impl Into<String>) {
        self.status = Status::warning(message);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.status = Status::error(message);
    }

    pub fn disaster(&mut self, message: impl Into<String>) {
        self.status = Status::disaster(message);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub overall: Status,
    pub locations: ComponentStatus,
    pub asns: ComponentStatus,
    pub geo: ComponentStatus,
    pub blocklist: ComponentStatus,
}

impl AppStatus {
    /// The most severe status across db and all components.
    pub fn worst(&self) -> Status {
        [
            &self.locations.status,
            &self.asns.status,
            &self.geo.status,
            &self.blocklist.status,
        ]
        .into_iter()
        .max_by_key(|s| s.status_code)
        .cloned()
        .unwrap_or_default()
    }

    pub fn iplist_error(&mut self, message: &str) {
        self.locations.error(message);
        self.asns.error(message);
        self.geo.error(message);
        self.blocklist.error(message);
    }

    pub fn iplist_ok(&mut self, message: &str) {
        self.locations.ok(message);
        self.asns.ok(message);
        self.geo.ok(message);
    }
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            overall: Status::never_updated(),
            locations: ComponentStatus::default(),
            asns: ComponentStatus::default(),
            geo: ComponentStatus::default(),
            blocklist: ComponentStatus::default(),
        }
    }
}

pub struct Schedule {
    pub blocklist_cron: Cron,
    pub iplist_cron: Cron,
}

impl Schedule {
    pub fn new(blocklist_cron: &str, iplist_cron: &str) -> Result<Self, AppError> {
        Ok(Self {
            blocklist_cron: Self::build(blocklist_cron)?,
            iplist_cron: Self::build(iplist_cron)?,
        })
    }

    pub fn build(cron: &str) -> Result<Cron, AppError> {
        let schedule = croner::parser::CronParser::builder()
            .seconds(croner::parser::Seconds::Required)
            .dom_and_dow(true)
            .build()
            .parse(cron)
            .map_err(|e| AppError::ParseError(e.to_string()))?;
        Ok(schedule)
    }

    pub fn get_next_run_blocklist(&self) -> Option<OffsetDateTime> {
        Self::next_run(&self.blocklist_cron)
    }

    pub fn get_next_run_iplist(&self) -> Option<OffsetDateTime> {
        Self::next_run(&self.iplist_cron)
    }

    fn next_run(cron: &Cron) -> Option<OffsetDateTime> {
        let next = cron
            .find_next_occurrence(&chrono::Local::now(), false)
            .ok()?;
        let instant = OffsetDateTime::from_unix_timestamp(next.timestamp()).ok()?;
        let offset = UtcOffset::from_whole_seconds(next.offset().local_minus_utc()).ok()?;
        Some(instant.to_offset(offset))
    }
}
