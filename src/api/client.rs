use std::collections::HashMap;

use crate::api::{ApiData, ApiError};
use reqwest::{Error, RequestBuilder, Response, header::HeaderMap};
use serde::Serialize;
use serde_json::Error as SerdeError;
use spdlog::prelude::*;
use thiserror::Error;
use url::Url;

#[derive(Debug)]
pub struct ApiClient {
    base_url: String,
    // TODO: remove warning
    #[allow(dead_code)]
    token: String,
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

    #[error("json error")]
    ParseError(#[from] SerdeError),

    #[error("missing data in response")]
    MissingData,
}

impl ApiClient {
    pub fn new(base_url: String, token: String) -> Result<Self, ClientError> {
        let api_url = Url::parse(&base_url);

        if let Err(e) = api_url {
            return Err(ClientError::BadUrl(e));
        }

        Ok(ApiClient {
            base_url,
            token,
            client: reqwest::Client::new(),
        })
    }

    pub async fn get(
        &self,
        uri: &str,
        headers: Option<HeaderMap>,
    ) -> Result<ApiData<serde_json::Value>, ClientError> {
        let url = format!("{}{}", self.base_url, uri);
        let request = self.client.get(url);

        self.send(request, headers).await
    }

    // TODO: remove warning
    #[allow(dead_code)]
    pub async fn post<T: Serialize>(
        &self,
        uri: &str,
        headers: Option<HeaderMap>,
        body: &T,
    ) -> Result<ApiData<serde_json::Value>, ClientError> {
        let url = format!("{}{}", self.base_url, uri);
        let request = self.client.post(url).json(body);

        self.send(request, headers).await
    }

    pub async fn patch<T: Serialize + std::fmt::Debug>(
        &self,
        uri: &str,
        headers: Option<HeaderMap>,
        body: &T,
    ) -> Result<ApiData<serde_json::Value>, ClientError> {
        let url = format!("{}{}", self.base_url, uri);
        let request = self.client.patch(url).json(body);

        self.send(request, headers).await
    }

    async fn send(
        &self,
        mut request: RequestBuilder,
        headers: Option<HeaderMap>,
    ) -> Result<ApiData<serde_json::Value>, ClientError> {
        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        let res = request.send().await?;
        self.handle_response(res).await
    }

    async fn handle_response(
        &self,
        response: Response,
    ) -> Result<ApiData<serde_json::Value>, ClientError> {
        let status = response.status();
        let message = response.text().await?;
        let body: HashMap<String, serde_json::Value> =
            serde_json::from_str(&message).map_err(ClientError::ParseError)?;

        if status.is_client_error() || status.is_server_error() {
            let mut error_messages = Vec::new();
            if let Some(errors) = body.get("errors").and_then(|v| v.as_array()) {
                for err in errors {
                    let detail = err
                        .get("detail")
                        .and_then(|d| d.as_str())
                        .unwrap_or_default();
                    error_messages.push(detail.to_string());
                }
            }
            let combined_message = error_messages.join("; ");
            return Err(ClientError::ApiError(ApiError::new(
                status,
                combined_message,
            )));
        }

        let mut api_response: ApiData<serde_json::Value> = ApiData::new();

        if status.is_success()
            && let Some(data) = body.get("data")
        {
            let value = match data {
                serde_json::Value::Array(arr) => {
                    // Extract attributes from each element in the array
                    let extracted_attrs: Vec<serde_json::Value> = arr
                        .iter()
                        .map(|item| match item.get("attributes") {
                            Some(attrs) => attrs.clone(),
                            None => item.clone(),
                        })
                        .collect();
                    serde_json::Value::Array(extracted_attrs)
                }
                serde_json::Value::Object(obj) => obj
                    .get("attributes")
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(Default::default())),
                _ => data.clone(),
            };
            api_response.data = Some(value);
        }

        Ok(api_response)
    }
}

// TODO: remove this or improve
impl Default for ApiClient {
    fn default() -> Self {
        // Provide dummy values just to satisfy the trait
        ApiClient {
            base_url: String::new(),
            token: String::new(),
            client: reqwest::Client::new(),
        }
    }
}
