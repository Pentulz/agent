use std::{
    process::Command,
    time::{Duration, SystemTime},
};

use crate::{agent::Agent, tool::Tool};

pub struct Job {
    started_at: SystemTime,
    ended_at: SystemTime,
    timeout: Duration,
    tool: Tool,
    agent_id: u32,
}

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
}
