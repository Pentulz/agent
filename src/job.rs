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

use crate::{action::Action, report::Report};

#[derive(Clone)]
pub struct Job {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Arc<Mutex<Option<DateTime<Utc>>>>,
    action: Action,
    agent_id: Uuid,
    result: Arc<Mutex<Option<Report>>>,
    submitted: Arc<AtomicBool>,
}

impl Job {
    // used by unit tests
    #[allow(dead_code)]
    pub fn new(name: String, cmd: String, args: Vec<String>) -> Self {
        Job {
            id: Uuid::new_v4(),
            name: name.to_string(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: Arc::new(Mutex::new(None)),
            action: Action::new(cmd, args),
            agent_id: Uuid::new_v4(),
            result: Arc::new(Mutex::new(None)),
            submitted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn was_submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn set_submitted(&self, val: bool) {
        self.submitted.store(val, Ordering::Relaxed)
    }

    pub fn run(&self) -> Result<String, std::io::Error> {
        self.action.run()
    }

    pub fn get_action(&self) -> &Action {
        &self.action
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn set_result(&self, val: Report) {
        let mut guard = self.result.lock().unwrap();
        *guard = Some(val);
    }

    pub fn set_completed_at(&self) {
        let mut completed_guard = self.completed_at.lock().unwrap();
        *completed_guard = Some(Utc::now());
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
            .field("created_at", &self.created_at)
            .field("started_at", &self.started_at)
            .field("completed_at", &self.completed_at)
            .field("action", &self.action)
            .field("agent_id", &self.agent_id)
            .field("results", &self.result)
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
        s.serialize_field("created_at", &self.created_at.to_rfc3339())?;
        s.serialize_field("started_at", &self.started_at.map(|t| t.to_rfc3339()))?;

        let completed_at_guard = self.completed_at.lock().unwrap();
        s.serialize_field(
            "completed_at",
            &completed_at_guard.as_ref().map(|t| t.to_rfc3339()),
        )?;

        s.serialize_field("action", &self.action)?;
        s.serialize_field("agent_id", &self.agent_id)?;
        let output_guard = self.result.lock().unwrap();
        s.serialize_field("results", &*output_guard)?;
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
            created_at: DateTime<Utc>,
            started_at: Option<DateTime<Utc>>,
            completed_at: Option<DateTime<Utc>>,
            action: Action,
            agent_id: Uuid,
            output: Option<Report>,
        }

        let helper = JobHelper::deserialize(deserializer)?;
        Ok(Job {
            id: helper.id,
            name: helper.name,
            created_at: helper.created_at,
            started_at: helper.started_at,
            completed_at: Arc::new(Mutex::new(helper.completed_at)),
            action: helper.action,
            agent_id: helper.agent_id,
            result: Arc::new(Mutex::new(helper.output)),
            submitted: Arc::new(AtomicBool::new(false)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::str::FromStr;
    use uuid::Uuid;

    // Simple fake Report for testing
    fn make_report() -> Report {
        Report {
            id: Uuid::new_v4(),
            results: serde_json::json!({"ok": true}),
            created_at: Utc::now(),
        }
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
        assert_eq!(job.result.lock().unwrap().as_ref().unwrap().id, report.id);
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
