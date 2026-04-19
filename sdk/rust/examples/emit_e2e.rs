use agentichound_trace::client::CollectorClient;
use agentichound_trace::contract_v0::{
    Event, IngestRequest, Run, Span, SpanKind, Status, TraceDocument, Usage, UsageKind,
};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

fn build_request() -> (String, IngestRequest) {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis();
    let run_id = format!("run_demo_{suffix}");
    let span_id = format!("spn_demo_{suffix}");
    let event_id = format!("evt_demo_{suffix}");
    let usage_id = format!("use_demo_{suffix}");
    let batch_id = format!("bat_demo_{suffix}");

    let started_at = "2026-04-20T00:00:00.000Z".to_string();
    let ended_at = "2026-04-20T00:00:01.000Z".to_string();
    let sent_at = "2026-04-20T00:00:02.000Z".to_string();

    let trace = TraceDocument {
        schema_version: "v0".to_string(),
        run: Run {
            run_id: run_id.clone(),
            started_at: started_at.clone(),
            ended_at: ended_at.clone(),
            duration_ms: 1000,
            status: Status::Ok,
        },
        spans: vec![Span {
            span_id: span_id.clone(),
            run_id: run_id.clone(),
            parent_span_id: None,
            retry_of_span_id: None,
            kind: SpanKind::Orchestration,
            name: "agent.execute".to_string(),
            started_at: started_at.clone(),
            ended_at: ended_at.clone(),
            duration_ms: 1000,
            status: Status::Ok,
        }],
        events: vec![Event {
            event_id,
            run_id: run_id.clone(),
            span_id: span_id.clone(),
            kind: SpanKind::Orchestration,
            ts: "2026-04-20T00:00:00.500Z".to_string(),
            payload: json!({"phase":"start"}),
        }],
        errors: vec![],
        usage: vec![Usage {
            usage_id,
            run_id: run_id.clone(),
            span_id,
            kind: UsageKind::Model,
            prompt_tokens: 12,
            completion_tokens: 8,
            total_tokens: 20,
            estimated_cost_usd: 0.0002,
            currency: "USD".to_string(),
        }],
    };

    let request = IngestRequest {
        batch_id,
        sent_at,
        traces: vec![trace],
    };

    (run_id, request)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url =
        std::env::var("COLLECTOR_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let client = CollectorClient::new(base_url)?;
    let (run_id, request) = build_request();

    let ingest = client.ingest(&request).await?;
    println!(
        "ingest accepted={} replayed={} ingestion_id={}",
        ingest.accepted, ingest.replayed, ingest.ingestion_id
    );

    let runs = client.runs(Some(100), None).await?;
    let listed = runs.runs.iter().any(|r| r.run_id == run_id);
    if !listed {
        return Err("run_id not found in /v0/runs response".into());
    }
    println!("run is visible in /v0/runs: {}", run_id);

    let detail = client.run(&run_id).await?;
    if detail.meta.run_id != run_id {
        return Err("run detail run_id mismatch".into());
    }
    println!(
        "run detail retrieved: run_id={} status={:?}",
        detail.meta.run_id, detail.trace.run.status
    );

    Ok(())
}
