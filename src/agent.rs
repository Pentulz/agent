use std::sync::Arc;

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Deserializer;
use serde::Serializer;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use spdlog::{debug, error};

use crate::api::client::ClientError;
use crate::job::Job;
use crate::{api::ApiClient, tool::Tool};
use serde_json::Error as SerdeError;

use gethostname::gethostname;

#[derive(Debug, Serialize, Deserialize)]
enum AgentPlatform {
    Linux,
    MacOs,
    Windows,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Agent {
    id: Option<String>,
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    auth_token: String,
    #[serde(
        serialize_with = "serialize_jobs",
        deserialize_with = "deserialize_jobs"
    )]
    jobs: Arc<Mutex<Vec<Arc<Job>>>>,
    hostname: Option<String>,
    #[serde(
        serialize_with = "serialize_platform",
        deserialize_with = "deserialize_platform"
    )]
    platform: Option<AgentPlatform>,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,

    #[serde(skip)]
    client: ApiClient,
    available_tools: Option<Vec<Tool>>,
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

fn serialize_platform<S>(platform: &Option<AgentPlatform>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match platform {
        Some(AgentPlatform::Linux) => serializer.serialize_some("linux"),
        Some(AgentPlatform::MacOs) => serializer.serialize_some("macos"),
        Some(AgentPlatform::Windows) => serializer.serialize_some("windows"),
        None => serializer.serialize_none(),
    }
}

fn deserialize_platform<'de, D>(deserializer: D) -> Result<Option<AgentPlatform>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct PlatformVisitor;

    impl<'de> Visitor<'de> for PlatformVisitor {
        type Value = Option<AgentPlatform>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a platform string (linux, macos, windows) or null")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value.to_lowercase().as_str() {
                "linux" => Ok(Some(AgentPlatform::Linux)),
                "macos" | "mac" => Ok(Some(AgentPlatform::MacOs)),
                "windows" | "win" => Ok(Some(AgentPlatform::Windows)),
                _ => Err(de::Error::unknown_variant(
                    value,
                    &["linux", "macos", "windows"],
                )),
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_option(PlatformVisitor)
}

impl Agent {
    pub async fn new(base_url: String, auth_token: String) -> Result<Agent, ClientError> {
        let mut client = ApiClient::new(base_url, auth_token.clone())?;

        let mut agent = Agent::get_by_id(&mut client, &auth_token).await?;
        agent.platform = Agent::get_platform();
        agent.hostname = Some(Agent::get_hostname());
        agent.client = client;

        Ok(agent)
    }

    // TODO:
    #[allow(dead_code)]
    pub async fn register(&mut self) -> Result<(), ClientError> {
        let uri = format!("/agents/{}", self.id.clone().unwrap());
        self.last_seen_at = Some(Utc::now());

        self.client.patch(&uri, None, &self).await?;

        Ok(())
    }

    pub async fn get_by_id(client: &mut ApiClient, id: &str) -> Result<Agent, ClientError> {
        let uri = format!("/agents/{}", id);
        let res = client.get(&uri, None).await?;
        let data = res.data.unwrap();
        let parsed: Result<Agent, SerdeError> = serde_json::from_str(&data);

        match parsed {
            Ok(agent) => Ok(agent),

            Err(e) => {
                error!("JSON parse error: {}", e);
                Err(ClientError::JsonError(e))
            }
        }
    }

    pub async fn check_health(&self) -> Result<(), ClientError> {
        let result = self.client.get("/health", None).await;

        match result {
            Ok(_api_data) => Ok(()),
            Err(err) => Err(err),
        }
    }

    #[allow(dead_code)]
    pub async fn fetch_tools(&self) -> Result<Vec<Tool>, ClientError> {
        let res = self.client.get("/tools", None).await?;
        let parsed: Vec<Tool> = serde_json::from_str(&res.data.unwrap()).unwrap();

        debug!("data: {:?}", parsed);

        Ok(parsed)
    }

    pub async fn get_jobs(&mut self) -> Result<(), ClientError> {
        let uri = format!("/agents/{}/jobs", self.id.clone().unwrap());
        let res = self.client.get(&uri, None).await?;
        let jobs: Vec<Job> = serde_json::from_str(&res.data.unwrap()).unwrap();

        if !jobs.is_empty() {
            let mut guard = self.jobs.lock().unwrap();
            guard.extend(jobs.into_iter().map(Arc::new));
        }

        Ok(())
    }

    pub async fn run_jobs(&self) -> Result<(), ClientError> {
        let jobs = {
            let guard = self.jobs.lock().unwrap();
            guard.clone() // clone Vec<Arc<Job>> (increase ref instead of cloning whole struct)
        };

        let futures = jobs
            .into_iter()
            .map(|job| tokio::task::spawn(async move { job.run() }));

        let results = futures::future::join_all(futures).await;

        for res in results {
            match res {
                Ok(Ok(output)) => debug!("Job output: {}", output),
                Ok(Err(job_err)) => error!("Job error: {}", job_err),
                Err(join_err) => error!("Task join error (panic/cancel): {}", join_err),
            }
        }

        Ok(())
    }

    async fn get_tools(&self) -> Result<Vec<Tool>, ClientError> {
        let uri = "/tools";
        let res = self.client.get(uri, None).await?;
        let tools: Vec<Tool> = serde_json::from_str(&res.data.unwrap()).unwrap();

        Ok(tools)
    }

    pub async fn get_available_tools(&self) -> Result<Vec<Tool>, ClientError> {
        let available_tools: Vec<Tool> = self
            .get_tools()
            .await?
            .into_iter()
            .filter(|tool| tool.is_available())
            .collect();

        Ok(available_tools)
    }

    pub async fn submit_capabilities(&mut self) -> Result<(), ClientError> {
        self.available_tools = Some(self.get_available_tools().await?);
        let uri = format!("/agents/{}", self.id.clone().unwrap());

        self.client.patch(&uri, None, &self).await?;

        Ok(())
    }

    #[allow(dead_code)]
    fn submit_report() {
        // TODO: send a report of a task like: agent_id, tool_id (NULL if tool not available), output
        todo!("");
    }

    fn get_hostname() -> String {
        gethostname().to_string_lossy().into_owned()
    }

    fn get_platform() -> Option<AgentPlatform> {
        match std::env::consts::OS {
            "linux" => Some(AgentPlatform::Linux),
            "macos" => Some(AgentPlatform::MacOs),
            "windows" => Some(AgentPlatform::Windows),
            _ => None, // Unknown OS
        }
    }
}
