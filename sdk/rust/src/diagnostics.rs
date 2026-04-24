use crate::contract_v0::{SpanKind, Status, TraceDocument};
use serde::Serialize;
use std::collections::HashSet;

pub const PROGRESS_COLLAPSE_DIAGNOSTIC: &str = "progress_collapse_detector";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportingSignals {
    pub span_count: usize,
    pub non_retry_span_count: usize,
    pub retry_span_count: usize,
    pub error_count: usize,
    pub usage_count: usize,
    pub unique_successful_step_count: usize,
    pub repeated_successful_step_count: usize,
    pub repeated_span_ratio: f64,
    pub retry_ratio: f64,
    pub duration_ms: i64,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressCollapseDiagnostic {
    pub run_id: String,
    pub diagnostic: &'static str,
    pub severity: DiagnosticSeverity,
    pub reasons: Vec<String>,
    pub supporting_signals: SupportingSignals,
    pub summary: String,
}

pub fn diagnose_progress_collapse(trace: &TraceDocument) -> ProgressCollapseDiagnostic {
    let span_count = trace.spans.len();
    let retry_span_count = trace
        .spans
        .iter()
        .filter(|span| matches!(&span.kind, SpanKind::Retry))
        .count();
    let non_retry_span_count = span_count.saturating_sub(retry_span_count);
    let error_count = trace.errors.len();
    let usage_count = trace.usage.len();
    let total_tokens = trace
        .usage
        .iter()
        .map(|usage| usage.total_tokens.max(0))
        .sum::<i64>();
    let total_cost_usd = trace
        .usage
        .iter()
        .map(|usage| usage.estimated_cost_usd.max(0.0))
        .sum::<f64>();

    let successful_non_retry_spans = trace
        .spans
        .iter()
        .filter(|span| !matches!(&span.kind, SpanKind::Retry) && matches!(&span.status, Status::Ok))
        .collect::<Vec<_>>();

    let mut successful_step_keys = HashSet::new();
    for span in &successful_non_retry_spans {
        successful_step_keys.insert(step_key(&span.kind, &span.name));
    }

    let unique_successful_step_count = successful_step_keys.len();
    let repeated_successful_step_count = successful_non_retry_spans
        .len()
        .saturating_sub(unique_successful_step_count);

    let repeated_span_ratio = if non_retry_span_count > 0 {
        repeated_successful_step_count as f64 / non_retry_span_count as f64
    } else {
        0.0
    };
    let retry_ratio = if span_count > 0 {
        retry_span_count as f64 / span_count as f64
    } else {
        0.0
    };

    let duration_ms = trace.run.duration_ms;
    let high_activity_hits = [
        duration_ms >= 10_000,
        span_count >= 10,
        retry_span_count >= 2,
        error_count >= 1,
        usage_count > 0 && total_tokens >= 2_000,
    ]
    .into_iter()
    .filter(|hit| *hit)
    .count();

    let low_progress_hits = [
        unique_successful_step_count <= 2 && span_count >= 8,
        repeated_span_ratio >= 0.40,
        retry_ratio >= 0.20,
        error_count >= 2,
        duration_ms >= 30_000 && unique_successful_step_count <= 3,
    ]
    .into_iter()
    .filter(|hit| *hit)
    .count();

    let severity = if high_activity_hits >= 4 && low_progress_hits >= 4 {
        DiagnosticSeverity::Critical
    } else if high_activity_hits >= 3 && low_progress_hits >= 3 {
        DiagnosticSeverity::High
    } else if high_activity_hits >= 2 && low_progress_hits >= 2 {
        DiagnosticSeverity::Medium
    } else {
        DiagnosticSeverity::Low
    };

    let mut reasons = Vec::new();
    if duration_ms >= 10_000 {
        reasons.push("long runtime".to_string());
    }
    if span_count >= 10 {
        reasons.push("high span volume".to_string());
    }
    if retry_span_count >= 2 {
        reasons.push("retry-heavy execution pattern".to_string());
    }
    if error_count >= 1 {
        reasons.push("errors present in the run".to_string());
    }
    if usage_count > 0 && total_tokens >= 2_000 {
        reasons.push("high token usage".to_string());
    }
    if unique_successful_step_count <= 2 && span_count >= 8 {
        reasons.push("few distinct successful steps relative to total activity".to_string());
    }
    if repeated_span_ratio >= 0.40 {
        reasons.push("repeated work dominates the non-retry span mix".to_string());
    }
    if retry_ratio >= 0.20 {
        reasons.push("retry pressure is high relative to total spans".to_string());
    }
    if error_count >= 2 {
        reasons.push("multiple errors with limited forward movement".to_string());
    }
    if duration_ms >= 30_000 && unique_successful_step_count <= 3 {
        reasons.push("long runtime with little new successful step diversity".to_string());
    }
    if reasons.is_empty() {
        reasons.push("clear forward movement with limited retry or error pressure".to_string());
    }

    let summary = match severity {
        DiagnosticSeverity::Low => "Run does not show collapse pressure.".to_string(),
        DiagnosticSeverity::Medium => {
            "Run is busy enough to inspect for progress collapse.".to_string()
        }
        DiagnosticSeverity::High => {
            "Run showed sustained activity but weak forward movement.".to_string()
        }
        DiagnosticSeverity::Critical => {
            "Run strongly resembles progress collapse with repeated work and limited progress."
                .to_string()
        }
    };

    ProgressCollapseDiagnostic {
        run_id: trace.run.run_id.clone(),
        diagnostic: PROGRESS_COLLAPSE_DIAGNOSTIC,
        severity,
        reasons,
        supporting_signals: SupportingSignals {
            span_count,
            non_retry_span_count,
            retry_span_count,
            error_count,
            usage_count,
            unique_successful_step_count,
            repeated_successful_step_count,
            repeated_span_ratio,
            retry_ratio,
            duration_ms,
            total_tokens,
            total_cost_usd,
        },
        summary,
    }
}

fn step_key(kind: &SpanKind, name: &str) -> String {
    format!("{kind:?}:{name}")
}
