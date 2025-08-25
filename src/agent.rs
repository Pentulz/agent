use std::string::ParseError;

use reqwest::Error;

use crate::api::client::ClientError;
use crate::api::{ApiClient, ApiError};
use crate::task::Task;

pub struct Agent {
    client: ApiClient,
    auth_token: String,
    checksum: String,
    tasks: Vec<Task>,
}

impl Agent {
    pub fn new(base_url: String, auth_token: String) -> Result<Agent, ClientError> {
        let client = ApiClient::new(base_url, auth_token)?;

        Ok(Agent {
            auth_token: "".to_string(),
            checksum: "".to_string(),
            tasks: vec![],
            client,
        })
    }

    pub async fn check_health(&self) -> Result<(), ClientError> {
        self.client.check_health().await?;
        Ok(())
    }

    fn fetch_tasks() {}

    fn run_tasks() {}

    fn register() {
        // TODO: make request to /register to fetch info from db
        // (like deployment_id and checksum) and then perform local check
        // of that checksum
    }

    fn fetch_capabilities() {
        // TODO: get list of cmds that will be run by an agent
        // and check if they exist
    }

    fn submit_capabilities() {
        // TODO: inform the backend which tools are available to an agent
    }

    fn submit_report() {
        // TODO: send a report of a task like: agent_id, tool_id (NULL if tool not available), output
    }
}
