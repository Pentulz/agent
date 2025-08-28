use std::collections::HashMap;

use crate::api::{ApiData, ApiError};
use reqwest::{Body, Error, RequestBuilder, Response, header::HeaderMap};
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

    #[error("json error")]
    JsonError(#[from] SerdeError),
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
        headers: Option<HeaderMap>,
    ) -> Result<ApiData<String>, ClientError> {
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
    ) -> Result<ApiData<String>, ClientError> {
        let url = format!("{}{}", self.base_url, uri);
        let request = self.client.post(url).json(body);

        self.send(request, headers).await
    }

    pub async fn patch<T: Serialize>(
        &self,
        uri: &str,
        headers: Option<HeaderMap>,
        body: &T,
    ) -> Result<ApiData<String>, ClientError> {
        let url = format!("{}{}", self.base_url, uri);
        let request = self.client.patch(url).json(body);

        self.send(request, headers).await
    }

    async fn send(
        &self,
        mut request: RequestBuilder,
        headers: Option<HeaderMap>,
    ) -> Result<ApiData<String>, ClientError> {
        // TODO: add auth token

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        let res = request.send().await?;
        self.handle_response(res).await
    }

    async fn handle_response(&self, response: Response) -> Result<ApiData<String>, ClientError> {
        let status = response.status();
        let message = response.text().await?;
        let body: HashMap<String, serde_json::Value> = serde_json::from_str(&message).unwrap();

        if status.is_client_error() || status.is_server_error() {
            // TODO: maybe rename errors to error
            return Err(ClientError::ApiError(ApiError::new(
                status,
                body["errors"][0]["title"].to_string(),
                body["errors"][0]["detail"].to_string(),
            )));
        }

        let mut api_response: ApiData<String> = ApiData::new();
        if status.is_success() {
            api_response.data = Some(body["data"].to_string());
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
            auth_token: String::new(),
            client: reqwest::Client::new(),
        }
    }
}
