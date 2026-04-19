use crate::types::{
    IngestRequest, SpanKind, TraceDocument, MAX_ERRORS_PER_TRACE, MAX_EVENTS_PER_TRACE,
    MAX_SPANS_PER_TRACE, MAX_TOTAL_ENTITIES_PER_REQUEST, MAX_TRACES_PER_REQUEST,
    MAX_USAGE_PER_TRACE,
};
use chrono::{DateTime, Utc};
use jsonschema::JSONSchema;
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::OnceLock;

fn batch_id_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^bat_[A-Za-z0-9_-]+$").expect("valid regex"))
}

fn trace_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let schema_raw = include_str!("../../schemas/trace.schema.json");
        let schema_json: Value =
            serde_json::from_str(schema_raw).expect("trace schema must be valid JSON");
        JSONSchema::compile(&schema_json).expect("trace schema must compile")
    })
}

fn parse_ts(ts: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| format!("invalid timestamp: {ts}"))
}

pub fn validate_request_limits(req: &IngestRequest) -> Result<(), String> {
    if !batch_id_regex().is_match(&req.batch_id) {
        return Err("batch_id format invalid".to_string());
    }

    parse_ts(&req.sent_at)?;

    if req.traces.is_empty() {
        return Err("traces must contain at least one trace".to_string());
    }
    if req.traces.len() > MAX_TRACES_PER_REQUEST {
        return Err("traces exceeds per-request limit".to_string());
    }

    let mut total_entities = 0usize;
    for trace in &req.traces {
        if trace.spans.len() > MAX_SPANS_PER_TRACE {
            return Err("spans exceeds per-trace limit".to_string());
        }
        if trace.events.len() > MAX_EVENTS_PER_TRACE {
            return Err("events exceeds per-trace limit".to_string());
        }
        if trace.errors.len() > MAX_ERRORS_PER_TRACE {
            return Err("errors exceeds per-trace limit".to_string());
        }
        if trace.usage.len() > MAX_USAGE_PER_TRACE {
            return Err("usage exceeds per-trace limit".to_string());
        }
        total_entities +=
            1 + trace.spans.len() + trace.events.len() + trace.errors.len() + trace.usage.len();
    }
    if total_entities > MAX_TOTAL_ENTITIES_PER_REQUEST {
        return Err("total entities exceeds request limit".to_string());
    }

    Ok(())
}

pub fn validate_trace(trace: &TraceDocument) -> Result<(), String> {
    let as_value = serde_json::to_value(trace).map_err(|e| e.to_string())?;
    if let Err(mut errs) = trace_schema().validate(&as_value) {
        if let Some(first) = errs.next() {
            return Err(format!("schema validation failed: {first}"));
        }
    }

    let run = &trace.run;
    let run_start = parse_ts(&run.started_at)?;
    let run_end = parse_ts(&run.ended_at)?;
    let run_ms = (run_end - run_start).num_milliseconds();
    if run_ms != run.duration_ms {
        return Err("run duration_ms mismatch".to_string());
    }

    let span_ids: HashSet<String> = trace.spans.iter().map(|s| s.span_id.clone()).collect();
    for span in &trace.spans {
        if span.run_id != run.run_id {
            return Err("span run_id mismatch".to_string());
        }
        if let Some(parent) = &span.parent_span_id {
            if !span_ids.contains(parent) {
                return Err("span parent_span_id not found".to_string());
            }
        }
        if let Some(retry_of) = &span.retry_of_span_id {
            if !span_ids.contains(retry_of) {
                return Err("span retry_of_span_id not found".to_string());
            }
        }
        match span.kind {
            SpanKind::Retry => {
                if span.retry_of_span_id.is_none() {
                    return Err("retry span missing retry_of_span_id".to_string());
                }
            }
            _ => {
                if span.retry_of_span_id.is_some() {
                    return Err("non-retry span cannot set retry_of_span_id".to_string());
                }
            }
        }
        let span_start = parse_ts(&span.started_at)?;
        let span_end = parse_ts(&span.ended_at)?;
        let span_ms = (span_end - span_start).num_milliseconds();
        if span_ms != span.duration_ms {
            return Err("span duration_ms mismatch".to_string());
        }
        if span_start < run_start || span_end > run_end {
            return Err("span outside run time window".to_string());
        }
    }

    for event in &trace.events {
        if event.run_id != run.run_id {
            return Err("event run_id mismatch".to_string());
        }
        if !span_ids.contains(&event.span_id) {
            return Err("event span_id not found".to_string());
        }
        let _ = parse_ts(&event.ts)?;
    }
    for err in &trace.errors {
        if err.run_id != run.run_id {
            return Err("error run_id mismatch".to_string());
        }
        if !span_ids.contains(&err.span_id) {
            return Err("error span_id not found".to_string());
        }
        let _ = parse_ts(&err.ts)?;
    }
    for usage in &trace.usage {
        if usage.run_id != run.run_id {
            return Err("usage run_id mismatch".to_string());
        }
        if !span_ids.contains(&usage.span_id) {
            return Err("usage span_id not found".to_string());
        }
        if usage.total_tokens != usage.prompt_tokens + usage.completion_tokens {
            return Err("usage total_tokens mismatch".to_string());
        }
    }

    Ok(())
}
