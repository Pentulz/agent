use std::sync::Arc;

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Deserializer;
use serde::Serializer;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use spdlog::{debug, error};
use uuid::Uuid;

use crate::api::client::ClientError;
use crate::job::Job;
use crate::report::Report;
use crate::{api::ApiClient, tool::Tool};

use gethostname::gethostname;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum AgentPlatform {
    Linux,
    MacOS,
    Windows,
}

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
    hostname: Option<String>,
    description: Option<String>,
    platform: Option<AgentPlatform>,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,

    available_tools: Option<Vec<Tool>>,

    #[serde(skip)]
    client: ApiClient,
}

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

        let mut agent = Agent::get_by_id(&mut client, &token).await?;
        agent.platform = Agent::get_platform();
        agent.hostname = Some(Agent::get_hostname());
        agent.client = client;

        Ok(agent)
    }

    #[allow(dead_code)]
    pub fn available_tools(&self) -> &Option<Vec<Tool>> {
        &self.available_tools
    }

    pub async fn register(&mut self) -> Result<(), ClientError> {
        let uri = format!("/agents/{}", self.id.unwrap());
        self.last_seen_at = Some(Utc::now());

        self.client.patch(&uri, None, &self).await?;

        Ok(())
    }

    pub async fn get_by_id(client: &mut ApiClient, id: &str) -> Result<Agent, ClientError> {
        let uri = format!("/agents/{}", id);
        let res = client.get(&uri, None).await?;
        let data = res.data.unwrap();
        let agent: Agent = serde_json::from_value(data).map_err(ClientError::ParseError)?;

        Ok(agent)
    }

    pub async fn check_health(&self) -> Result<(), ClientError> {
        let result = self.client.get("/health", None).await;

        match result {
            Ok(_api_data) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get_jobs(&mut self) -> Result<(), ClientError> {
        let uri = format!("/agents/{}/jobs", self.id.unwrap());
        let res = self.client.get(&uri, None).await?;
        let jobs: Vec<Job> = serde_json::from_value(res.data.unwrap()).unwrap();

        if !jobs.is_empty() {
            let mut guard = self.jobs.lock().unwrap();
            guard.extend(jobs.into_iter().map(Arc::new));
        }

        Ok(())
    }

    pub async fn run_jobs(&self) -> Result<(), ClientError> {
        let jobs = {
            let guard = self.jobs.lock().unwrap();
            guard.clone() // Vec<Arc<Job>>
        };

        let futures = jobs.into_iter().map(|job| {
            tokio::task::spawn(async move {
                match job.run() {
                    Ok(output) => {
                        debug!("JOB finished, creating Report...");

                        let report = Report {
                            id: Uuid::new_v4(),
                            results: serde_json::json!({ "output": output }),
                            created_at: Utc::now(),
                        };

                        job.set_result(report.clone());
                        job.set_completed();

                        Ok(report)
                    }
                    Err(err) => {
                        let report = Report {
                            id: Uuid::new_v4(),
                            results: serde_json::json!({ "error": err.to_string() }),
                            created_at: Utc::now(),
                        };

                        job.set_result(report.clone());
                        job.set_completed();

                        Err(err)
                    }
                }
            })
        });

        let results = futures::future::join_all(futures).await;

        for res in results {
            match res {
                Ok(Ok(report)) => debug!("Job report: {:?}", report),
                Ok(Err(job_err)) => error!("Job error: {}", job_err),
                Err(join_err) => error!("Task join error (panic/cancel): {}", join_err),
            }
        }

        Ok(())
    }

    async fn get_tools(&self) -> Result<Vec<Tool>, ClientError> {
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
            .map(|item| {
                let attrs = item.get("attributes").ok_or(ClientError::MissingData)?; // or another custom error
                serde_json::from_value(attrs.clone()).map_err(ClientError::ParseError)
            })
            .collect();

        tools
    }

    pub async fn get_available_tools(&self) -> Result<Vec<Tool>, ClientError> {
        let mut available_tools: Vec<Tool> = self
            .get_tools()
            .await?
            .into_iter()
            .filter(|tool| tool.is_available())
            .collect();

        for tool in available_tools.iter_mut() {
            if tool.version().is_none() {
                tool.get_version();
            }
        }

        Ok(available_tools)
    }

    pub async fn submit_capabilities(&mut self) -> Result<(), ClientError> {
        self.available_tools = Some(self.get_available_tools().await?);
        let uri = format!("/agents/{}", self.id.unwrap());

        debug!(
            "submit_capabilities(available_tools) => {:?}",
            &self.available_tools
        );

        self.client.patch(&uri, None, &self).await?;

        Ok(())
    }

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
            let uri = format!("/jobs/{}", job.get_id());
            self.client.patch(&uri, None, &*job).await?;
            job.set_submitted(true);
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
