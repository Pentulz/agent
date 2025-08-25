use crate::api::ApiError;
use reqwest::{Error, Response, header::HeaderMap};
use thiserror::Error;
use url::Url;

pub struct ApiClient {
    base_url: String,
    auth_token: String,
    client: reqwest::Client,
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("bad base url")]
    BadUrl(#[from] url::ParseError),

    #[error("api error")]
    ApiError(#[from] ApiError),

    #[error("reqwest error")]
    ReqwestError(#[from] Error),
}

impl ApiClient {
    pub fn new(base_url: String, auth_token: String) -> Result<Self, ClientError> {
        let api_url = Url::parse(&base_url);

        if let Err(e) = api_url {
            return Err(ClientError::BadUrl(e));
        }

        Ok(ApiClient {
            base_url,
            auth_token,
            client: reqwest::Client::new(),
        })
    }

    pub async fn get(
        &self,
        uri: &str,
        query_string: Option<String>,
        headers: Option<HeaderMap>,
    ) -> Result<(), ClientError> {
        let url = format!("{}{}", self.base_url, uri);

        println!("checking url: {}", url);

        let mut request = self.client.get(url);

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        let res = request.send().await?;
        self.handle_response(res).await
    }

    pub async fn check_health(&self) -> Result<(), ClientError> {
        self.get("/health", None, None).await
    }

    async fn handle_response(&self, response: Response) -> Result<(), ClientError> {
        let status = response.status();
        let message = response.text().await?;

        if status.is_client_error() {
            return Err(ClientError::ApiError(ApiError::new(
                crate::api::error::ErrorCode::BadRequest,
                message,
            )));
        }

        println!("res: {}", message);

        Ok(())
    }

    // pub async fn health_check(&self) -> Result<bool, ApiError> {}

    // pub async fn register_agent(&self, agent_info: &AgentInfo) -> Result<(), ApiError> {}
    //
    // pub async fn fetch_tasks(&self) -> Result<Vec<TaskData>, ApiError> {}
    //
    // pub async fn get_task(&self, task_id: &str) -> Result<TaskData, ApiError> {}
    //
    // pub async fn submit_result(&self, result: &TaskResult) -> Result<(), ApiError> {}
    //
    // pub async fn heartbeat(&self, agent_id: &str) -> Result<(), ApiError> {}
}
