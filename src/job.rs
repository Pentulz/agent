use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use chrono::{DateTime, Duration, Utc};

use crate::tool::Tool;

#[derive(Serialize, Deserialize)]
pub struct Job {
    id: String,
    name: String,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    #[serde(
        serialize_with = "serialize_chrono_duration",
        deserialize_with = "deserialize_chrono_duration"
    )]
    timeout: Duration,
    tool: Tool,
    agent_id: u32,
}

// TODO: remove warning
#[allow(dead_code)]
impl Job {
    pub fn new(cmd: String, args: Vec<String>, timeout: Duration) -> Job {
        // TODO: implement properly started_at and ended_at later
        Job {
            id: "".to_string(),
            name: "".to_string(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            timeout,
            tool: Tool::new(cmd, args),
            agent_id: 0,
        }
    }

    pub fn run(&self) -> Result<String, std::io::Error> {
        self.tool.run()
    }

    fn submit_report() {
        // TODO: send a report of a task like: agent_id, tool_id (NULL if tool not available), output
        todo!("");
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
            .finish()
    }
}

fn serialize_chrono_duration<S>(dur: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(dur.num_seconds()) // serialize as seconds
}

fn deserialize_chrono_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = i64::deserialize(deserializer)?;
    Ok(Duration::seconds(secs))
}
