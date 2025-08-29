use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use uuid::Uuid;

use chrono::{DateTime, Utc};

use crate::{report::Report, tool::Tool};

#[derive(Clone)]
pub struct Job {
    id: String,
    name: String,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Arc<Mutex<Option<DateTime<Utc>>>>,
    action: Tool,
    agent_id: Uuid,
    result: Arc<Mutex<Option<Report>>>,
    submitted: Arc<AtomicBool>,
}

impl Job {
    pub fn was_submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn set_submitted(&self, val: bool) {
        self.submitted.store(val, Ordering::Relaxed)
    }

    pub fn run(&self) -> Result<String, std::io::Error> {
        self.action.run()
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn set_result(&self, val: Report) {
        let mut guard = self.result.lock().unwrap();
        *guard = Some(val);
    }

    pub fn set_completed(&self) {
        let mut completed_guard = self.completed_at.lock().unwrap();
        *completed_guard = Some(Utc::now());
    }
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("created_at", &self.created_at)
            .field("started_at", &self.started_at)
            .field("completed_at", &self.completed_at)
            .field("tool", &self.action)
            .field("agent_id", &self.agent_id)
            .field("results", &self.result)
            .finish()
    }
}

impl Serialize for Job {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("Job", 8)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("name", &self.name)?;
        // s.serialize_field("created_at", &self.created_at)?;
        // s.serialize_field("started_at", &self.started_at)?;
        s.serialize_field("created_at", &self.created_at.to_rfc3339())?;
        s.serialize_field("started_at", &self.started_at.map(|t| t.to_rfc3339()))?;

        let completed_at_guard = self.completed_at.lock().unwrap();
        // s.serialize_field("completed_at", &*completed_at_guard.)?;
        s.serialize_field(
            "completed_at",
            &completed_at_guard.as_ref().map(|t| t.to_rfc3339()),
        )?;

        s.serialize_field("action", &self.action)?;
        s.serialize_field("agent_id", &self.agent_id)?;
        // serialize output as Option<String>
        let output_guard = self.result.lock().unwrap();
        s.serialize_field("results", &*output_guard)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for Job {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct JobHelper {
            id: String,
            name: String,
            created_at: DateTime<Utc>,
            started_at: Option<DateTime<Utc>>,
            completed_at: Option<DateTime<Utc>>,
            action: Tool,
            agent_id: Uuid,
            output: Option<Report>,
        }

        let helper = JobHelper::deserialize(deserializer)?;
        Ok(Job {
            id: helper.id,
            name: helper.name,
            created_at: helper.created_at,
            started_at: helper.started_at,
            completed_at: Arc::new(Mutex::new(helper.completed_at)),
            action: helper.action,
            agent_id: helper.agent_id,
            result: Arc::new(Mutex::new(helper.output)),
            submitted: Arc::new(AtomicBool::new(false)),
        })
    }
}
