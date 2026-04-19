use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct Run {
    pub run_id: String,
    pub started_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub kind: SpanKind,
    pub name: String,
    pub status: SpanStatus,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Copy)]
pub enum SpanKind {
    Orchestration,
    Model,
    Tool,
    Retry,
    Handoff,
    Approval,
}

#[derive(Debug, Clone, Copy)]
pub enum SpanStatus {
    Ok,
    Error,
    Cancelled,
}

#[derive(Debug, Default)]
pub struct Tracer;

impl Tracer {
    pub fn start_run(&self, run_id: impl Into<String>) -> Run {
        Run {
            run_id: run_id.into(),
            started_at: SystemTime::now(),
        }
    }
}
