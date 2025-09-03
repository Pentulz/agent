use std::env;
use std::fmt::Display;
#[cfg(unix)]
use std::fs;
use std::process::Command;

use serde::{Deserialize, Serialize};
use spdlog::{debug, error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tool {
    cmd: String,
    version: Option<String>,
    version_arg: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("missing version_arg for {0}")]
    MissingVersionArg(String),

    #[error("failed to run {0}: {1}")]
    CommandFailed(String, #[source] std::io::Error),

    #[error("utf8 decode failed")]
    Utf8Error,
}

impl Tool {
    #[allow(dead_code)]
    pub fn new(cmd: String) -> Tool {
        let mut tool = Tool {
            cmd,
            version: None,
            version_arg: None,
        };

        debug!("Getting tool version...");
        if let Err(err) = tool.get_version() {
            error!("{}: {}", tool, err);
        }
        debug!("Finished");

        tool
    }

    pub fn get_version(&mut self) -> Result<(), ToolError> {
        let version_arg = self
            .version_arg
            .clone()
            .ok_or_else(|| ToolError::MissingVersionArg(self.cmd.clone()))?;

        let output = Command::new(&self.cmd)
            .arg(version_arg)
            .output()
            .map_err(|e| ToolError::CommandFailed(self.cmd.clone(), e))?;

        let version = String::from_utf8(output.stdout).map_err(|_| ToolError::Utf8Error)?;
        self.version = Some(version);
        Ok(())
    }

    pub fn version(&self) -> &Option<String> {
        &self.version
    }

    pub fn is_available(&self) -> bool {
        if let Some(paths) = env::var_os("PATH") {
            for path in env::split_paths(&paths) {
                let full_path = path.join(&self.cmd);

                #[cfg(unix)]
                {
                    if let Ok(metadata) = fs::metadata(&full_path) {
                        use std::os::unix::fs::PermissionsExt;
                        let mode = metadata.permissions().mode();
                        if mode & 0o111 != 0 {
                            return true; // has any execute bit
                        }
                    }
                }

                #[cfg(windows)]
                {
                    if full_path.exists() {
                        return true; // existence is usually enough on Windows
                    }
                }
            }
        }
        false
    }
}

impl Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cmd: {}, version: {:#?}, version_arg: {:#?}",
            self.cmd, self.version, self.version_arg
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_is_available_true_for_known_command() {
        // "echo" exists on Unix, "cmd" exists on Windows
        #[cfg(unix)]
        let cmd = "echo".to_string();
        #[cfg(windows)]
        let cmd = "cmd".to_string();

        let tool = Tool {
            cmd,
            version: None,
            version_arg: None,
        };

        assert!(tool.is_available());
    }

    #[test]
    fn test_tool_is_available_false_for_nonexistent_command() {
        let tool = Tool {
            cmd: "non_existing_cmd".to_string(),
            version: None,
            version_arg: None,
        };

        assert!(!tool.is_available());
    }

    #[test]
    fn test_get_version_for_echo() {
        // "echo" prints back its argument, so we can use it as a fake "version command"
        #[cfg(unix)]
        let mut tool = Tool {
            cmd: "echo".to_string(),
            version: None,
            version_arg: Some("--version".to_string()),
        };
        #[cfg(windows)]
        let mut tool = Tool {
            cmd: "cmd".to_string(),
            version: None,
            version_arg: Some("/C ver".to_string()), // "ver" prints Windows version
        };

        let _ = tool.get_version();

        assert!(tool.version().is_some());
        assert!(!tool.version().as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_new_does_not_panic_even_if_version_arg_none() {
        // Here we construct with just the binary name
        // It won't set version, but must not panic
        let _tool = Tool::new("echo".to_string());
    }
}
