mod normalize;
mod storage;
pub mod types;
mod validate;

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use normalize::canonical_json;
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};
use storage::{Storage, StorageError};
use types::{
    ErrorDetail, ErrorResponse, IngestRequest, RunDetailResponse, RunsResponse, MAX_PAYLOAD_BYTES,
};
use validate::{validate_request_limits, validate_trace};

#[derive(Clone)]
pub struct AppState {
    storage: Arc<Mutex<Storage>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::from_db_path(Self::default_db_path()).expect("failed to create default app state")
    }

    pub fn default_db_path() -> PathBuf {
        let from_env = std::env::var("COLLECTOR_DB_PATH").ok();
        from_env
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("collector.db"))
    }

    pub fn from_db_path(path: impl AsRef<FsPath>) -> Result<Self, String> {
        let storage = Storage::open(path).map_err(|e| e.to_string())?;
        Ok(Self {
            storage: Arc::new(Mutex::new(storage)),
        })
    }

    pub fn in_memory() -> Self {
        let storage = Storage::in_memory().expect("failed to create in-memory storage");
        Self {
            storage: Arc::new(Mutex::new(storage)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/v0/ingest", post(post_ingest))
        .route("/v0/runs", get(get_runs))
        .route("/v0/runs/:run_id", get(get_run_by_id))
        .layer(DefaultBodyLimit::max(MAX_PAYLOAD_BYTES))
        .with_state(state)
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
    details: Vec<ErrorDetail>,
}

impl ApiError {
    fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.to_string(),
            message: message.into(),
            details: Vec::new(),
        }
    }

    fn with_detail(mut self, path: impl Into<String>, reason: impl Into<String>) -> Self {
        self.details.push(ErrorDetail {
            path: path.into(),
            reason: reason.into(),
        });
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            accepted: false,
            code: self.code,
            message: self.message,
            details: self.details,
        };
        (self.status, Json(body)).into_response()
    }
}

impl From<StorageError> for ApiError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::BatchIdConflict => ApiError::new(
                StatusCode::CONFLICT,
                "BATCH_ID_CONFLICT",
                "batch_id reused with different payload",
            ),
            StorageError::EntityConflict(_) => {
                ApiError::new(StatusCode::CONFLICT, "ENTITY_CONFLICT", value.to_string())
            }
            StorageError::SchemaVersionMismatch { .. } => ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                value.to_string(),
            ),
            StorageError::Sqlite(msg) => {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg)
            }
            StorageError::Serialize(msg) | StorageError::Deserialize(msg) => {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg)
            }
        }
    }
}

fn require_json_content_type(headers: &HeaderMap) -> Result<(), ApiError> {
    let value = headers.get(header::CONTENT_TYPE).ok_or_else(|| {
        ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "UNSUPPORTED_MEDIA_TYPE",
            "content-type must be application/json",
        )
    })?;
    let as_str = value.to_str().map_err(|_| {
        ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "UNSUPPORTED_MEDIA_TYPE",
            "invalid content-type header",
        )
    })?;
    let is_json = as_str.eq_ignore_ascii_case("application/json")
        || as_str.to_ascii_lowercase().starts_with("application/json;");
    if !is_json {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "UNSUPPORTED_MEDIA_TYPE",
            "content-type must be application/json",
        ));
    }
    Ok(())
}

fn decode_ingest_request(body: &[u8]) -> Result<(IngestRequest, String), ApiError> {
    let value: Value = serde_json::from_slice(body).map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "MALFORMED_JSON",
            "request body is not valid JSON",
        )
    })?;
    let req: IngestRequest = serde_json::from_value(value.clone()).map_err(|err| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "VALIDATION_ERROR",
            "invalid ingest envelope",
        )
        .with_detail("envelope", err.to_string())
    })?;
    let body_canonical = canonical_json(&value).map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            format!("failed to normalize request: {err}"),
        )
    })?;
    Ok((req, body_canonical))
}

async fn post_ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Result<impl IntoResponse, ApiError> {
    require_json_content_type(&headers)?;
    let body = body.map_err(|rejection| {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE",
                "request payload exceeds max payload size",
            )
        } else {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "MALFORMED_JSON",
                "request body is not valid JSON",
            )
        }
    })?;
    if body.len() > MAX_PAYLOAD_BYTES {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "PAYLOAD_TOO_LARGE",
            "request payload exceeds max payload size",
        ));
    }
    let (req, body_canonical) = decode_ingest_request(&body)?;
    validate_request_limits(&req).map_err(|err| {
        ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", err)
            .with_detail("envelope", "limits or format")
    })?;
    for (i, trace) in req.traces.iter().enumerate() {
        validate_trace(trace).map_err(|err| {
            ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", err)
                .with_detail(format!("traces[{i}]"), "trace validation")
        })?;
    }
    let mut storage = state.storage.lock().map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "store lock poisoned",
        )
    })?;
    // Idempotency/conflict comparisons use deterministic canonical JSON, not raw input bytes.
    let (status, response) = storage.ingest(&req, &body_canonical)?;
    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    Ok((status_code, Json(response)))
}

#[derive(Debug, Deserialize)]
struct RunsQuery {
    limit: Option<usize>,
    cursor: Option<String>,
}

fn parse_cursor(cursor: &str) -> Option<usize> {
    let body = cursor.strip_prefix("c_")?;
    body.parse::<usize>().ok()
}

async fn get_runs(
    State(state): State<AppState>,
    Query(query): Query<RunsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_QUERY",
            "limit must be between 1 and 100",
        ));
    }
    let offset = match query.cursor {
        None => 0,
        Some(cursor) => parse_cursor(&cursor).ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_QUERY",
                "cursor format invalid",
            )
        })?,
    };
    let storage = state.storage.lock().map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "store lock poisoned",
        )
    })?;
    let response: RunsResponse = storage.list_runs(limit, offset)?;
    Ok((StatusCode::OK, Json(response)))
}

async fn get_run_by_id(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let storage = state.storage.lock().map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "store lock poisoned",
        )
    })?;
    let maybe: Option<RunDetailResponse> = storage.get_run(&run_id)?;
    let Some(response) = maybe else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "RUN_NOT_FOUND",
            "run_id not found",
        ));
    };
    Ok((StatusCode::OK, Json(response)))
}
