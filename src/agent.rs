use std::sync::Arc;

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Deserializer;
use serde::Serializer;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use spdlog::info;
use spdlog::{debug, error};

use crate::api::client::ClientError;
use crate::job::Job;
use crate::job::JobPatch;
use crate::{api::ApiClient, tool::Tool};

use gethostname::gethostname;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
enum AgentPlatform {
    Linux,
    MacOS,
    Windows,
}

/// Structs to map required JSON payload of API endpoints

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentCapabilities {
    available_tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentPresence {
    last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum RunJobsError {
    #[error("job failed: {0}")]
    JobFailed(String),

    #[error("one or more jobs failed")]
    AtLeastOneFailed(Vec<RunJobsError>),

    #[error("tokio join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("mutex poisoned")]
    Mutex,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRegister {
    platform: Option<AgentPlatform>,
    hostname: Option<String>,
    last_seen_at: Option<DateTime<Utc>>,
}

/// Main agents structure. It maps the agent's table on the BD + has some required fields
/// to properly handle running jobs in background (async)
#[derive(Debug, Serialize, Deserialize)]
pub struct Agent {
    id: Option<uuid::Uuid>,
    #[allow(dead_code)]
    token: String,
    #[serde(
        serialize_with = "serialize_jobs",
        deserialize_with = "deserialize_jobs"
    )]
    jobs: Arc<Mutex<Vec<Arc<Job>>>>,
    name: String,
    hostname: Option<String>,
    description: Option<String>,
    platform: Option<AgentPlatform>,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,

    available_tools: Option<Vec<Tool>>,

    #[serde(skip)]
    client: ApiClient,
}

/// Serde JSON serialization and deserialization methods
fn serialize_jobs<S>(jobs: &Arc<Mutex<Vec<Arc<Job>>>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let jobs = jobs.lock().map_err(serde::ser::Error::custom)?;
    let mut seq = serializer.serialize_seq(Some(jobs.len()))?;
    for job in jobs.iter() {
        seq.serialize_element(&**job)?; // &Arc<Job> → &Job
    }
    seq.end()
}

type SharedJobs = Arc<Mutex<Vec<Arc<Job>>>>;
fn deserialize_jobs<'de, D>(deserializer: D) -> Result<SharedJobs, D::Error>
where
    D: Deserializer<'de>,
{
    let jobs_vec = Vec::<Job>::deserialize(deserializer)?;
    Ok(Arc::new(Mutex::new(
        jobs_vec.into_iter().map(Arc::new).collect(),
    )))
}

impl Agent {
    pub async fn new(base_url: String, token: String) -> Result<Agent, ClientError> {
        let mut client = ApiClient::new(base_url, token.clone())?;

        let mut agent = Agent::get_info(&mut client).await?;
        agent.platform = Agent::get_platform();
        agent.hostname = Some(Agent::get_hostname());
        agent.client = client;

        Ok(agent)
    }

    #[allow(dead_code)]
    pub fn available_tools(&self) -> &Option<Vec<Tool>> {
        &self.available_tools
    }

    // performs PATCH /self
    pub async fn announce_presence(&mut self) -> Result<(), ClientError> {
        info!("Announcing presence...");
        let uri = "/self";
        self.last_seen_at = Some(Utc::now());

        let agent = AgentPresence {
            last_seen_at: self.last_seen_at,
        };

        self.client.patch(uri, None, &agent).await?;
        info!("Finished");

        Ok(())
    }

    // performs PATCH /self to update agent's hostname, platform and last_seen_at
    pub async fn register(&mut self) -> Result<(), ClientError> {
        info!("Registring agent...");
        let uri = "/self";
        self.last_seen_at = Some(Utc::now());

        let agent = AgentRegister {
            hostname: self.hostname.clone(),
            platform: self.platform.clone(),
            last_seen_at: self.last_seen_at,
        };

        self.client.patch(uri, None, &agent).await?;
        info!("Done");

        Ok(())
    }

    // performs GET /self to fetch agent's info at the startup of this daemon
    pub async fn get_info(client: &mut ApiClient) -> Result<Agent, ClientError> {
        let uri = "/self";
        let res = client.get(uri, None).await?;
        let data = res.data.unwrap();
        let agent: Agent = serde_json::from_value(data).map_err(ClientError::ParseError)?;

        Ok(agent)
    }

    // performs GET /jobs to fetch agent's jobs
    pub async fn get_jobs(&mut self) -> Result<(), ClientError> {
        info!("Fetching jobs...");

        let uri = "/jobs";
        let res = self.client.get(uri, None).await?;
        let jobs: Vec<Job> = serde_json::from_value(res.data.unwrap()).unwrap();

        if !jobs.is_empty() {
            let mut guard = self.jobs.lock().unwrap();
            guard.extend(jobs.into_iter().map(Arc::new));
        }

        info!("Finished");

        Ok(())
    }

    // run jobs in background using tokio's futures and Arc + Mutexes to ensure the Agent structure
    // is thread-safe
    pub async fn run_jobs(&self) -> Result<(), RunJobsError> {
        let jobs = {
            let guard = self.jobs.lock().map_err(|_| RunJobsError::Mutex)?;
            // really make sure we do not rerun jobs that are already  running in the background
            guard
                .iter()
                .filter(|job| job.get_started_at().is_none() && job.get_completed_at().is_none())
                .cloned()
                .collect::<Vec<_>>() // only fresh jobs
        };

        // launch jobs in background
        let futures = jobs.into_iter().map(|job| {
            info!("Running job: {}", &job);
            tokio::task::spawn(async move {
                match job.run() {
                    Ok(output) => {
                        info!("Job {} finished, creating Report...", job.get_id());
                        job.set_result(output.clone());
                        job.set_completed_at();
                        job.set_success(true);

                        Ok(output)
                    }
                    Err(err) => {
                        job.set_result(err.to_string());
                        job.set_completed_at();
                        job.set_success(false);
                        Err(RunJobsError::JobFailed(format!(
                            "Job {} failed, {}: {}",
                            &job,
                            job.get_action(),
                            err
                        )))
                    }
                }
            })
        });

        // wait for all jobs and start to fetch their output to return them
        let results = futures::future::join_all(futures).await;
        let mut reports = Vec::new();
        let mut errors = Vec::new();

        for res in results {
            match res {
                Ok(Ok(report)) => {
                    debug!("OK(OK(report)) => pushing report");
                    reports.push(report);
                }
                Ok(Err(job_err)) => {
                    debug!("OK(Err(job_err)) => pushing errors");
                    errors.push(job_err);
                }
                Err(join_err) => {
                    debug!("Err(join_err)) => join error");
                    errors.push(RunJobsError::Join(join_err));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.remove(0))
        } else {
            Err(RunJobsError::AtLeastOneFailed(errors))
        }
    }

    // perform GET /tools to fetch available tools on the API so the agent can check its own
    // available tools (capabilities)
    async fn get_tools(&self) -> Result<Vec<Tool>, ClientError> {
        debug!("Getting tools...");
        let uri = "/tools";
        let res = self.client.get(uri, None).await?;

        let data = res.data.ok_or(ClientError::MissingData)?;

        // Make sure it's an array
        let tools_array = match data {
            serde_json::Value::Array(ref arr) => arr,
            _ => return Ok(vec![]), // or Err if preferred
        };

        // Map each element's "attributes" to Tool
        let tools: Result<Vec<Tool>, ClientError> = tools_array
            .iter()
            .map(|item| serde_json::from_value(item.clone()).map_err(ClientError::ParseError))
            .collect();

        tools
    }

    // for each tool returned by the GET /tools, check locally if the agent has access to them
    pub async fn get_available_tools(&self) -> Result<Vec<Tool>, ClientError> {
        let mut available_tools: Vec<Tool> = self
            .get_tools()
            .await?
            .into_iter()
            .filter(|tool| tool.is_available())
            .collect();

        for tool in available_tools.iter_mut() {
            if tool.version().is_none() {
                let _ = tool.get_version();
            }
        }

        Ok(available_tools)
    }

    // perform PATCH /self to update its available_tools (capabilities)
    pub async fn submit_capabilities(&mut self) -> Result<(), ClientError> {
        info!("Submitting submit_capabilities...");
        self.available_tools = Some(self.get_available_tools().await?);

        let uri = "/self";
        let capabilities = AgentCapabilities {
            available_tools: self.available_tools.clone(),
        };

        self.client.patch(uri, None, &capabilities).await?;
        info!("Done");

        Ok(())
    }

    // perform PATCH /jobs/<id> to update job's output after executing it
    pub async fn submit_report(&mut self) -> Result<(), ClientError> {
        let jobs: Vec<Arc<Job>> = self
            .jobs
            .clone()
            .lock()
            .unwrap()
            .iter()
            .filter(|job| !job.was_submitted())
            .cloned()
            .collect();

        for job in jobs {
            info!("Submitting job report...");

            let uri = format!("/jobs/{}", job.get_id());
            job.set_submitted(true);

            let patch = JobPatch {
                started_at: job.get_started_at(),
                completed_at: job.get_completed_at(),
                results: job.get_result_as_string(),
                success: Some(job.is_success()),
            };

            self.client.patch(&uri, None, &patch).await?;
            info!("Finished!");
        }

        Ok(())
    }

    fn get_hostname() -> String {
        gethostname().to_string_lossy().into_owned()
    }

    fn get_platform() -> Option<AgentPlatform> {
        match std::env::consts::OS {
            "linux" => Some(AgentPlatform::Linux),
            "macos" => Some(AgentPlatform::MacOS),
            "windows" => Some(AgentPlatform::Windows),
            _ => None, // Unknown OS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    fn make_agent() -> Agent {
        Agent {
            id: Some(Uuid::new_v4()),
            token: "token".to_string(),
            jobs: Arc::new(Mutex::new(vec![])),
            name: "myname".to_string(),
            hostname: None,
            description: Some("Test agent".to_string()),
            platform: None,
            last_seen_at: None,
            created_at: Some(Utc::now()),
            available_tools: Some(vec![]),
            client: ApiClient::new("http://fake.url.com".to_string(), "fake_token".to_string())
                .unwrap(),
        }
    }

    fn make_jobs() -> Vec<Arc<Job>> {
        vec![
            Arc::new(Job::new(
                "echo_hello".to_string(),
                "echo".to_string(),
                ["Hello, world!".to_string()].to_vec(),
            )),
            Arc::new(Job::new(
                "sleep_1".to_string(),
                "sleep".to_string(),
                ["1".to_string()].to_vec(),
            )),
        ]
    }

    fn make_jobs_that_crash() -> Vec<Arc<Job>> {
        vec![
            Arc::new(Job::new(
                "echo_hello".to_string(),
                "echo1234".to_string(),
                ["Hello, world!".to_string()].to_vec(),
            )),
            Arc::new(Job::new(
                "sleep_1".to_string(),
                "sleep1234".to_string(),
                ["1".to_string()].to_vec(),
            )),
        ]
    }

    #[tokio::test]
    async fn test_get_hostname() {
        let hostname = Agent::get_hostname();
        assert!(!hostname.is_empty());
    }

    #[tokio::test]
    async fn test_get_platform() {
        let platform = Agent::get_platform();

        assert!(platform.is_some());
        assert!(matches!(
            platform,
            Some(AgentPlatform::Linux) | Some(AgentPlatform::MacOS) | Some(AgentPlatform::Windows)
        ));
    }

    #[tokio::test]
    async fn test_submit_jobs() {
        // Given
        let agent = make_agent();
        let jobs = make_jobs();

        // Prevent deadlock by the agent.run_jobs() function
        {
            let mut guard = agent.jobs.lock().unwrap();
            *guard = jobs;
        }

        // When
        let _result = agent.run_jobs().await;

        // Then
        let any_incompleted_job = agent
            .jobs
            .lock()
            .unwrap()
            .iter()
            .any(|job| !job.is_completed());

        assert!(!any_incompleted_job);
    }

    #[tokio::test]
    async fn test_submit_jobs_that_crash() {
        // Given
        let agent = make_agent();
        let jobs = make_jobs_that_crash();

        // Prevent deadlock by the agent.run_jobs() function
        {
            let mut guard = agent.jobs.lock().unwrap();
            *guard = jobs;
        }

        // When
        let result = agent.run_jobs().await;

        // Then
        assert!(result.is_err());
        if let Err(RunJobsError::AtLeastOneFailed(job_errors)) = result {
            assert_eq!(job_errors.len(), 2);
            assert!(
                job_errors
                    .iter()
                    .any(|e| matches!(e, RunJobsError::JobFailed(_)))
            );
        } else {
            panic!("Expected AtLeastOneFailed error variant");
        }
    }
}
