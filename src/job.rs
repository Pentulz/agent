use std::time::{Duration, SystemTime};

use crate::tool::Tool;

// TODO: remove warning
#[allow(dead_code)]
pub struct Job {
    started_at: SystemTime,
    ended_at: SystemTime,
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
            started_at: SystemTime::now(),
            ended_at: SystemTime::now(),
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
    }
}
