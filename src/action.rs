use std::{fmt::Display, process::Command};

use serde::{Deserialize, Serialize};
use spdlog::debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Action {
    cmd: String,
    args: Vec<String>,
}

impl Action {
    #[allow(dead_code)]
    pub fn new(cmd: String, args: Vec<String>) -> Self {
        Action { cmd, args }
    }

    pub fn run(&self) -> Result<String, std::io::Error> {
        debug!("Action.run(): {:?}", self.cmd);
        let output = Command::new(&self.cmd).args(&self.args).output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.cmd, self.args.join(" "))
    }
}
