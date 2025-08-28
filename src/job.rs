use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use chrono::{DateTime, Duration, Utc};

use crate::tool::Tool;

#[derive(Clone)]
pub struct Job {
    id: String,
    name: String,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    timeout: Duration,
    tool: Tool,
    agent_id: u32,
    output: Arc<Mutex<Option<String>>>,
    submitted: Arc<AtomicBool>,
}

impl Job {
    #[allow(dead_code)]
    pub fn new(cmd: String, args: Vec<String>, timeout: Duration) -> Job {
        Job {
            id: "".to_string(),
            name: "".to_string(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            timeout,
            tool: Tool::new(cmd, args),
            agent_id: 0,
            output: Arc::new(Mutex::new(None)),
            submitted: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn was_submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn set_submitted(&self, val: bool) {
        self.submitted.store(val, Ordering::Relaxed)
    }

    pub fn run(&self) -> Result<String, std::io::Error> {
        self.tool.run()
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn set_output(&self, val: String) {
        let mut guard = self.output.lock().unwrap();
        *guard = Some(val);
    }
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("started_at", &self.started_at)
            .field("ended_at", &self.ended_at)
            .field("timeout", &format!("{}s", self.timeout.num_seconds()))
            .field("tool", &self.tool)
            .field("agent_id", &self.agent_id)
            .field("output", &self.output)
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
        s.serialize_field("started_at", &self.started_at)?;
        s.serialize_field("ended_at", &self.ended_at)?;
        s.serialize_field("timeout", &self.timeout.num_seconds())?;
        s.serialize_field("tool", &self.tool)?;
        s.serialize_field("agent_id", &self.agent_id)?;
        // serialize output as Option<String>
        let output_guard = self.output.lock().unwrap();
        s.serialize_field("output", &*output_guard)?;
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
            started_at: DateTime<Utc>,
            ended_at: DateTime<Utc>,
            timeout: i64,
            tool: Tool,
            agent_id: u32,
            output: Option<String>,
        }

        let helper = JobHelper::deserialize(deserializer)?;
        Ok(Job {
            id: helper.id,
            name: helper.name,
            started_at: helper.started_at,
            ended_at: helper.ended_at,
            timeout: Duration::seconds(helper.timeout),
            tool: helper.tool,
            agent_id: helper.agent_id,
            output: Arc::new(Mutex::new(helper.output)),
            submitted: Arc::new(AtomicBool::new(false)),
        })
    }
}
