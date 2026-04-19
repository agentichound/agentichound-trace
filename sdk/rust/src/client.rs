use crate::contract_v0::{
    ErrorResponse, IngestRequest, IngestSuccessResponse, RunDetailResponse, RunsResponse,
};
use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api error {status}: {code} - {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
        details: Vec<String>,
    },
    #[error("invalid collector base url")]
    InvalidBaseUrl,
}

#[derive(Debug, Clone)]
pub struct CollectorClient {
    base_url: String,
    http: reqwest::Client,
}

impl CollectorClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, ClientError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(ClientError::InvalidBaseUrl);
        }
        Ok(Self {
            base_url,
            http: reqwest::Client::new(),
        })
    }

    pub async fn ingest(
        &self,
        request: &IngestRequest,
    ) -> Result<IngestSuccessResponse, ClientError> {
        let resp = self
            .http
            .post(format!("{}/v0/ingest", self.base_url))
            .json(request)
            .send()
            .await?;
        parse_response(resp).await
    }

    pub async fn runs(
        &self,
        limit: Option<usize>,
        cursor: Option<&str>,
    ) -> Result<RunsResponse, ClientError> {
        let mut query = Vec::new();
        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor.to_string()));
        }
        let resp = self
            .http
            .get(format!("{}/v0/runs", self.base_url))
            .query(&query)
            .send()
            .await?;
        parse_response(resp).await
    }

    pub async fn run(&self, run_id: &str) -> Result<RunDetailResponse, ClientError> {
        let resp = self
            .http
            .get(format!("{}/v0/runs/{run_id}", self.base_url))
            .send()
            .await?;
        parse_response(resp).await
    }
}

async fn parse_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, ClientError> {
    let status = resp.status();
    if status.is_success() {
        return resp.json::<T>().await.map_err(ClientError::Http);
    }
    parse_api_error(status, resp).await
}

async fn parse_api_error<T>(status: StatusCode, resp: reqwest::Response) -> Result<T, ClientError> {
    match resp.json::<ErrorResponse>().await {
        Ok(err) => Err(ClientError::Api {
            status: status.as_u16(),
            code: err.code,
            message: err.message,
            details: err
                .details
                .into_iter()
                .map(|d| format!("{}: {}", d.path, d.reason))
                .collect(),
        }),
        Err(parse_err) => Err(ClientError::Http(parse_err)),
    }
}
