use std::{fmt::Display, process::Command};

use serde::{Deserialize, Serialize};
use spdlog::debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Represents a command to execute with arguments and a variant label.
pub struct Action {
    cmd: String,
    args: Vec<String>,
    variant: String,
}

impl Action {
    pub fn new(cmd: String, args: Vec<String>) -> Self {
        Action {
            cmd,
            args,
            variant: "".to_string(),
        }
    }

    /// Executes the command with its arguments and returns the standard output as a String.
    pub fn run(&self) -> Result<String, std::io::Error> {
        debug!("Action.run(): {:?}", self.cmd);
        let output = Command::new(&self.cmd).args(&self.args).output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    #[allow(dead_code)]
    pub fn get_cmd(&self) -> &str {
        &self.cmd
    }

    #[allow(dead_code)]
    pub fn get_args(&self) -> &Vec<String> {
        &self.args
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.cmd, self.args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_action_new() {
        let action = Action::new("echo".to_string(), vec!["hello".to_string()]);
        assert_eq!(action.cmd, "echo");
        assert_eq!(action.args, vec!["hello"]);
    }

    #[tokio::test]
    async fn test_action_run_success() {
        let action = Action::new("echo".to_string(), vec!["hello".to_string()]);
        let output = action.run().unwrap();
        assert!(output.contains("hello"));
    }

    #[tokio::test]
    async fn test_action_run_failure() {
        let action = Action::new("nonexistent_command".to_string(), vec![]);
        let result = action.run();
        assert!(result.is_err());
        let err: io::Error = result.unwrap_err();
        // On Unix, kind should be NotFound
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_action_display() {
        let action = Action::new(
            "echo".to_string(),
            vec!["hello".to_string(), "world".to_string()],
        );
        let display_str = format!("{}", action);
        assert_eq!(display_str, "echo hello world");
    }
}
