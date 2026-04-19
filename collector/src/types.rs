use serde::{Deserialize, Serialize};

pub const MAX_PAYLOAD_BYTES: usize = 524_288;
pub const MAX_TRACES_PER_REQUEST: usize = 32;
pub const MAX_TOTAL_ENTITIES_PER_REQUEST: usize = 5_000;
pub const MAX_SPANS_PER_TRACE: usize = 1_000;
pub const MAX_EVENTS_PER_TRACE: usize = 2_000;
pub const MAX_ERRORS_PER_TRACE: usize = 200;
pub const MAX_USAGE_PER_TRACE: usize = 1_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IngestRequest {
    pub batch_id: String,
    pub sent_at: String,
    pub traces: Vec<TraceDocument>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TraceDocument {
    pub schema_version: String,
    pub run: Run,
    pub spans: Vec<Span>,
    pub events: Vec<Event>,
    pub errors: Vec<TraceError>,
    pub usage: Vec<Usage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub run_id: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
    pub status: Status,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpanKind {
    Model,
    Tool,
    Orchestration,
    Retry,
    Handoff,
    Approval,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub span_id: String,
    pub run_id: String,
    pub parent_span_id: Option<String>,
    pub retry_of_span_id: Option<String>,
    pub kind: SpanKind,
    pub name: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
    pub status: Status,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub event_id: String,
    pub run_id: String,
    pub span_id: String,
    pub kind: SpanKind,
    pub ts: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TraceError {
    pub error_id: String,
    pub run_id: String,
    pub span_id: String,
    pub ts: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageKind {
    Model,
    Tool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub usage_id: String,
    pub run_id: String,
    pub span_id: String,
    pub kind: UsageKind,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EntityCounts {
    pub traces: usize,
    pub runs: usize,
    pub spans: usize,
    pub events: usize,
    pub errors: usize,
    pub usage: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IngestSuccessResponse {
    pub accepted: bool,
    pub replayed: bool,
    pub ingestion_id: String,
    pub batch_id: String,
    pub received: EntityCounts,
    pub inserted: EntityCounts,
    pub duplicates_ignored: EntityCounts,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorDetail {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub accepted: bool,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<ErrorDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunSummary {
    pub run_id: String,
    pub status: Status,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
    pub span_count: usize,
    pub event_count: usize,
    pub error_count: usize,
    pub usage_count: usize,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
    pub last_ingested_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunsResponse {
    pub runs: Vec<RunSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunDetailMeta {
    pub run_id: String,
    pub ingested_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunDetailResponse {
    pub trace: TraceDocument,
    pub meta: RunDetailMeta,
}
