use std::process::Command;

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
}
