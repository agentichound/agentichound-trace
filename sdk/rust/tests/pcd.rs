use agentichound_trace::contract_v0::TraceDocument;
use agentichound_trace::diagnostics::{diagnose_progress_collapse, DiagnosticSeverity};

#[test]
fn happy_trace_is_low_severity() {
    let trace = load_happy_trace();
    let diagnostic = diagnose_progress_collapse(&trace);
    assert_eq!(diagnostic.run_id, "run_happy_1");
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Low);
    assert!(diagnostic
        .reasons
        .iter()
        .any(|reason| reason.contains("clear forward movement")));
    assert_eq!(diagnostic.supporting_signals.error_count, 0);
    assert_eq!(diagnostic.supporting_signals.retry_span_count, 0);
}

#[test]
fn pathological_trace_is_high_or_critical() {
    let trace = load_pathological_trace();
    let diagnostic = diagnose_progress_collapse(&trace);
    assert_eq!(diagnostic.run_id, "run_pcd_1");
    assert!(matches!(
        diagnostic.severity,
        DiagnosticSeverity::High | DiagnosticSeverity::Critical
    ));
    assert!(diagnostic.supporting_signals.duration_ms >= 30_000);
    assert!(diagnostic.supporting_signals.retry_span_count >= 2);
    assert!(diagnostic.supporting_signals.error_count >= 1);
    assert!(diagnostic
        .reasons
        .iter()
        .any(|reason| reason.contains("retry-heavy") || reason.contains("retry pressure")));
}

#[test]
fn retry_then_success_fixture_stays_low_with_retry_signal() {
    let trace = load_retry_then_success_trace();
    let diagnostic = diagnose_progress_collapse(&trace);
    assert_eq!(diagnostic.run_id, "run_retry_1");
    // Current fixtures are short and recover quickly, so this is intentionally low-severity.
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Low);
    assert!(diagnostic.supporting_signals.retry_ratio >= 0.20);
    assert!(diagnostic.supporting_signals.error_count <= 1);
    assert!(diagnostic
        .reasons
        .iter()
        .any(|reason| reason.contains("retry pressure")));
}

#[test]
fn failed_run_fixture_stays_low_with_error_signal() {
    let trace = load_failed_run_trace();
    let diagnostic = diagnose_progress_collapse(&trace);
    assert_eq!(diagnostic.run_id, "run_fail_1");
    // Existing failed fixtures are brief and not collapse-like under current thresholds.
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Low);
    assert!(diagnostic.supporting_signals.error_count >= 1);
    assert!(diagnostic.supporting_signals.duration_ms < 10_000);
    assert!(diagnostic
        .reasons
        .iter()
        .any(|reason| reason.contains("errors present")));
}

fn load_happy_trace() -> TraceDocument {
    let contents = include_str!("../../../schemas/examples/happy-path-01.json");
    serde_json::from_str(contents).expect("valid trace fixture")
}

fn load_pathological_trace() -> TraceDocument {
    let contents = include_str!("../../../schemas/examples/pcd-low-progress-sample.json");
    serde_json::from_str(contents).expect("valid trace fixture")
}

fn load_retry_then_success_trace() -> TraceDocument {
    let contents = include_str!("../../../schemas/examples/retry-then-success-01.json");
    serde_json::from_str(contents).expect("valid retry-then-success fixture")
}

fn load_failed_run_trace() -> TraceDocument {
    let contents = include_str!("../../../schemas/examples/failed-run-01.json");
    serde_json::from_str(contents).expect("valid failed-run fixture")
}
