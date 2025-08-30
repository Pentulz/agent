use std::{env, fs, process::Command};

use serde::{Deserialize, Serialize};
use spdlog::debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tool {
    cmd: String,
    version: Option<String>,
    version_arg: Option<String>,
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
        tool.get_version();
        debug!("Finished");

        tool
    }

    pub fn get_version(&mut self) {
        let output = Command::new(&self.cmd)
            .args(&[self.version_arg.clone().unwrap()])
            .output()
            .unwrap();

        let version = String::from_utf8_lossy(&output.stdout).to_string();
        self.version = Some(version);
    }

    pub fn version(&self) -> &Option<String> {
        &self.version
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
