use serde::{Deserialize, Serialize};
use spdlog::debug;

use crate::api::ApiClient;
use crate::api::client::ClientError;
use crate::job::Job;

// TODO: remove warning
#[allow(dead_code)]
pub struct Agent {
    client: ApiClient,
    auth_token: String,
    checksum: String,
    jobs: Vec<Job>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    cmd: String,
    args: Vec<String>,
}

// TODO: remove warning
#[allow(dead_code)]
impl Agent {
    pub fn new(base_url: String, auth_token: String) -> Result<Agent, ClientError> {
        let client = ApiClient::new(base_url, auth_token)?;

        Ok(Agent {
            auth_token: "".to_string(),
            checksum: "".to_string(),
            jobs: vec![],
            client,
        })
    }

    pub async fn check_health(&self) -> Result<(), ClientError> {
        let result = self.client.get("/health", None).await;

        match result {
            Ok(_api_data) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn fetch_tools(&self) -> Result<Vec<Tool>, ClientError> {
        let res = self.client.get("/tools", None).await?;
        let parsed: Vec<Tool> = serde_json::from_str(&res.data.unwrap()).unwrap();

        debug!("data: {:?}", parsed);

        Ok(parsed)
    }
    //
    fn fetch_jobs() {}

    fn run_jobs() {}

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
