use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Report {
    pub id: Uuid,
    pub results: Value,
    pub created_at: DateTime<Utc>,
}
