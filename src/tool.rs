use std::{env, fs, process::Command};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tool {
    cmd: String,
    args: Vec<String>,
}

impl Tool {
    pub fn new(cmd: String, args: Vec<String>) -> Tool {
        Tool { cmd, args }
    }

    pub fn run(&self) -> Result<String, std::io::Error> {
        let output = Command::new(&self.cmd).args(&self.args).output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn is_available(&self) -> bool {
        if let Some(paths) = env::var_os("PATH") {
            for path in env::split_paths(&paths) {
                let full_path = path.join(&self.cmd);
                if full_path.exists()
                    && let Ok(metadata) = fs::metadata(&full_path)
                {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mode = metadata.permissions().mode();
                        if mode & 0o111 != 0 {
                            return true; // has any execute bit
                        }
                    }
                    #[cfg(windows)]
                    {
                        // On Windows, existence is usually enough, executability is handled by extensions
                        return true;
                    }
                }
            }
        }
        false
    }
}
