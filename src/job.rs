use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use uuid::Uuid;

use chrono::{DateTime, Utc};

use crate::action::Action;

#[derive(Clone)]
pub struct Job {
    id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Arc<Mutex<Option<DateTime<Utc>>>>,
    completed_at: Arc<Mutex<Option<DateTime<Utc>>>>,
    action: Action,
    agent_id: Uuid,
    result: Arc<Mutex<Option<String>>>,
    submitted: Arc<AtomicBool>,
    success: Arc<Mutex<Option<bool>>>,
}

#[derive(Debug, Serialize)]
pub struct JobPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

impl Job {
    // used by unit tests
    #[allow(dead_code)]
    pub fn new(name: String, cmd: String, args: Vec<String>) -> Self {
        Job {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: Some("".to_string()),
            created_at: Utc::now(),
            started_at: Arc::new(Mutex::new(None)),
            completed_at: Arc::new(Mutex::new(None)),
            action: Action::new(cmd, args),
            agent_id: Uuid::new_v4(),
            result: Arc::new(Mutex::new(None)),
            submitted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            success: Arc::new(Mutex::new(Some(false))),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn new_internal(
        id: Uuid,
        name: String,
        description: Option<String>,
        created_at: DateTime<Utc>,
        started_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
        action: Action,
        agent_id: Uuid,
        result: Option<String>,
        success: Option<bool>,
    ) -> Self {
        Job {
            id,
            name,
            description,
            created_at,
            started_at: Arc::new(Mutex::new(started_at)),
            completed_at: Arc::new(Mutex::new(completed_at)),
            action,
            agent_id,
            result: Arc::new(Mutex::new(result)),
            submitted: Arc::new(AtomicBool::new(false)),
            success: Arc::new(Mutex::new(success)),
        }
    }

    pub fn was_submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn set_submitted(&self, val: bool) {
        self.submitted.store(val, Ordering::Relaxed)
    }

    pub fn run(&self) -> Result<String, std::io::Error> {
        {
            let mut guard = self.started_at.lock().unwrap();
            *guard = Some(Utc::now());
        }
        self.action.run()
    }

    pub fn get_action(&self) -> &Action {
        &self.action
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn set_result(&self, val: String) {
        let mut guard = self.result.lock().unwrap();
        *guard = Some(val);
    }

    pub fn set_completed_at(&self) {
        let mut completed_guard = self.completed_at.lock().unwrap();
        *completed_guard = Some(Utc::now());
    }

    pub fn set_sucess(&self, is_success: bool) {
        let mut guard = self.success.lock().unwrap();
        *guard = Some(is_success);
    }

    pub fn get_completed_at(&self) -> Option<DateTime<Utc>> {
        *self.completed_at.lock().unwrap()
    }

    pub fn get_started_at(&self) -> Option<DateTime<Utc>> {
        *self.started_at.lock().unwrap()
    }

    pub fn get_result_as_string(&self) -> Option<String> {
        self.result.lock().unwrap().as_ref().map(|r| r.to_string())
    }

    pub fn is_success(&self) -> bool {
        self.success.lock().unwrap().unwrap()
    }

    // used by unit tests
    #[allow(dead_code)]
    pub fn is_completed(&self) -> bool {
        self.completed_at.lock().unwrap().is_some() && self.result.lock().unwrap().is_some()
    }
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("created_at", &self.created_at)
            .field("started_at", &self.started_at)
            .field("completed_at", &self.completed_at)
            .field("action", &self.action)
            .field("agent_id", &self.agent_id)
            .field("results", &self.result)
            .field("success", &self.success)
            .finish()
    }
}

impl Serialize for Job {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("Job", 8)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("description", &self.description)?;
        s.serialize_field("created_at", &self.created_at.to_rfc3339())?;
        let started_at_guard = self.completed_at.lock().unwrap();
        s.serialize_field(
            "started_at",
            &started_at_guard.as_ref().map(|t| t.to_rfc3339()),
        )?;

        let completed_at_guard = self.completed_at.lock().unwrap();
        s.serialize_field(
            "completed_at",
            &completed_at_guard.as_ref().map(|t| t.to_rfc3339()),
        )?;

        s.serialize_field("action", &self.action)?;
        s.serialize_field("agent_id", &self.agent_id)?;
        let output_guard = self.result.lock().unwrap();
        s.serialize_field("results", &*output_guard)?;

        let success_guard = self.success.lock().unwrap();
        s.serialize_field("success", &*success_guard)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for Job {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct JobHelper {
            id: Uuid,
            name: String,
            description: Option<String>,
            agent_id: Uuid,
            created_at: DateTime<Utc>,
            started_at: Option<DateTime<Utc>>,
            completed_at: Option<DateTime<Utc>>,
            action: Action,
            result: Option<String>,
            success: Option<bool>,
        }

        let helper = JobHelper::deserialize(deserializer)?;
        Ok(Job::new_internal(
            helper.id,
            helper.name,
            helper.description,
            helper.created_at,
            helper.started_at,
            helper.completed_at,
            helper.action,
            helper.agent_id,
            helper.result,
            helper.success,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::str::FromStr;
    use uuid::Uuid;

    // Simple fake Report for testing
    fn make_report() -> String {
        format!(
            "{{\"id\": {}, \"results\": {}, \"created_at\": {}}}",
            Uuid::new_v4(),
            "ok",
            Utc::now(),
        )
    }

    #[test]
    fn test_job_new() {
        let job = Job::new(
            "test".to_string(),
            "echo".to_string(),
            vec!["hello".to_string()],
        );
        assert_eq!(job.name, "test");
        assert_eq!(job.get_action().get_cmd(), "echo");
        assert_eq!(job.get_action().get_args(), &vec!["hello"]);
        assert!(!job.was_submitted());
        assert!(job.completed_at.lock().unwrap().is_none());
        assert!(job.result.lock().unwrap().is_none());
    }

    #[test]
    fn test_set_and_check_submitted() {
        let job = Job::new("test".to_string(), "echo".to_string(), vec![]);
        assert!(!job.was_submitted());

        job.set_submitted(true);
        assert!(job.was_submitted());
    }

    #[test]
    fn test_set_result_and_completion() {
        let job = Job::new("test".to_string(), "echo".to_string(), vec![]);
        let report = make_report();

        job.set_result(report.clone());
        job.set_completed_at();

        assert!(job.is_completed());
        // assert_eq!(job.result.lock().unwrap().as_ref().unwrap().id, report.id);
        assert!(job.completed_at.lock().unwrap().is_some());
    }

    #[tokio::test]
    async fn test_run_action_success() {
        let job = Job::new(
            "test".to_string(),
            "echo".to_string(),
            vec!["hello".to_string()],
        );

        let output = job.run().unwrap();

        assert!(output.contains("hello"));
    }

    #[tokio::test]
    async fn test_run_action_failure() {
        let job = Job::new(
            "test".to_string(),
            "nonexistent_command".to_string(),
            vec![],
        );

        let result = job.run();

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialization() {
        let job = Job::new(
            "test".to_string(),
            "echo".to_string(),
            vec!["hi".to_string()],
        );

        let serialized = serde_json::to_string(&job).unwrap();
        let deserialized: Job = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, job.name);
        assert_eq!(
            deserialized.get_action().get_cmd(),
            job.get_action().get_cmd()
        );
        assert_eq!(
            deserialized.get_action().get_args(),
            job.get_action().get_args()
        );
    }

    #[test]
    fn test_debug_format() {
        let job = Job::new("test".to_string(), "echo".to_string(), vec![]);
        let debug_str = format!("{:?}", job);

        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("echo"));
    }

    #[test]
    fn test_deserialization_from_raw_json() {
        // Given a raw JSON string
        let raw_json = r#"
    {
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "name": "test",
        "created_at": "2025-08-28T12:41:34.061276Z",
        "agent_id": "550e8400-e29b-41d4-a716-446655440002",
        "action": {
            "cmd": "echo",
            "args": ["hi"]
        }
    }
    "#;

        // When
        let deserialized: Job = serde_json::from_str(raw_json).unwrap();

        // Then
        assert_eq!(deserialized.name, "test");
        assert_eq!(
            deserialized.id,
            Uuid::from_str("550e8400-e29b-41d4-a716-446655440001").unwrap()
        );
        assert_eq!(
            deserialized.agent_id,
            Uuid::from_str("550e8400-e29b-41d4-a716-446655440002").unwrap()
        );
        assert_eq!(deserialized.get_action().get_cmd(), "echo");
        assert_eq!(deserialized.get_action().get_args(), &vec!["hi"]);
    }
}
