use agentichound_trace::{SpanKind, SpanStatus, Tracer};

fn kind_label(kind: SpanKind) -> &'static str {
    match kind {
        SpanKind::Model => "model",
        SpanKind::Tool => "tool",
        SpanKind::Orchestration => "orchestration",
        SpanKind::Retry => "retry",
        SpanKind::Handoff => "handoff",
        SpanKind::Approval => "approval",
    }
}

fn status_label(status: SpanStatus) -> &'static str {
    match status {
        SpanStatus::Ok => "ok",
        SpanStatus::Error => "error",
        SpanStatus::Cancelled => "cancelled",
    }
}

#[test]
fn tracer_starts_run_with_expected_id() {
    let tracer = Tracer;
    let run = tracer.start_run("run_parity");
    assert_eq!(run.run_id, "run_parity");
}

#[test]
fn span_kind_set_matches_phase1_contract() {
    let kinds = [
        kind_label(SpanKind::Model),
        kind_label(SpanKind::Tool),
        kind_label(SpanKind::Orchestration),
        kind_label(SpanKind::Retry),
        kind_label(SpanKind::Handoff),
        kind_label(SpanKind::Approval),
    ];
    assert_eq!(
        kinds,
        [
            "model",
            "tool",
            "orchestration",
            "retry",
            "handoff",
            "approval"
        ]
    );
}

#[test]
fn status_set_matches_phase1_contract() {
    let statuses = [
        status_label(SpanStatus::Ok),
        status_label(SpanStatus::Error),
        status_label(SpanStatus::Cancelled),
    ];
    assert_eq!(statuses, ["ok", "error", "cancelled"]);
}
