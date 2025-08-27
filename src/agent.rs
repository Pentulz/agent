use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use spdlog::{debug, error};

use crate::api::client::ClientError;
use crate::job::Job;
use crate::{api::ApiClient, tool::Tool};
use serde_json::Error as SerdeError;

use serde::{Deserializer, Serializer};

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
    jobs: Vec<Job>,
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
}

fn serialize_jobs<S>(jobs: &Vec<Job>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(jobs.len()))?;
    for job in jobs {
        seq.serialize_element(job)?;
    }
    seq.end()
}

fn deserialize_jobs<'de, D>(deserializer: D) -> Result<Vec<Job>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<Job>::deserialize(deserializer)
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

// TODO: remove warning
#[allow(dead_code)]
impl Agent {
    pub async fn new(base_url: String, auth_token: String) -> Result<Agent, ClientError> {
        let mut client = ApiClient::new(base_url, auth_token.clone())?;

        let mut agent = Agent::get_by_id(&mut client, &auth_token).await?;
        agent.client = client;

        Ok(agent)
    }

    // TODO:
    pub async fn register(&self) -> Result<(), ClientError> {
        todo!("fetch hostname, platform and check capabilities");
    }

    pub async fn get_by_id(client: &mut ApiClient, id: &String) -> Result<Agent, ClientError> {
        let res = client.get(&format!("/agents/{}", id), None).await?;
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

    pub async fn fetch_tools(&self) -> Result<Vec<Tool>, ClientError> {
        let res = self.client.get("/tools", None).await?;
        let parsed: Vec<Tool> = serde_json::from_str(&res.data.unwrap()).unwrap();

        debug!("data: {:?}", parsed);

        Ok(parsed)
    }

    pub async fn fetch_jobs(&mut self) -> Result<(), ClientError> {
        todo!("fetch jobs from API and add them to the list of jobs");
    }

    fn run_jobs() {
        todo!("");
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
