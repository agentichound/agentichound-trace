use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use collector_core::{app, AppState};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

fn fixture(name: &str) -> String {
    let path = format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).expect("fixture should exist")
}

fn json_request(method: &str, path: &str, body: String) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("request")
}

async fn response_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("json body")
}

fn unique_db_path() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "agentichound_trace_test_{}_{}_{}.db",
        std::process::id(),
        ts,
        c
    ))
}

#[tokio::test]
async fn ingest_persists_across_restart_and_replay_semantics_hold() {
    let db_path = unique_db_path();
    let ingest_body = fixture("ingest-valid.json");

    let state1 = AppState::from_db_path(&db_path).expect("state1");
    let router1 = app(state1);
    let first = router1
        .clone()
        .oneshot(json_request("POST", "/v0/ingest", ingest_body.clone()))
        .await
        .expect("ingest");
    assert_eq!(first.status(), StatusCode::CREATED);

    let state2 = AppState::from_db_path(&db_path).expect("state2");
    let router2 = app(state2);

    let replay = router2
        .clone()
        .oneshot(json_request("POST", "/v0/ingest", ingest_body.clone()))
        .await
        .expect("replay");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_json = response_json(replay).await;
    assert_eq!(replay_json["replayed"], true);

    let runs = router2
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v0/runs?limit=10")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("runs");
    assert_eq!(runs.status(), StatusCode::OK);
    let runs_json = response_json(runs).await;
    assert_eq!(runs_json["runs"][0]["run_id"], "run_happy_1");

    let detail = router2
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v0/runs/run_happy_1")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("detail");
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_json = response_json(detail).await;
    assert_eq!(detail_json["trace"]["run"]["run_id"], "run_happy_1");

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn conflict_and_duplicate_behavior_persists_after_restart() {
    let db_path = unique_db_path();
    let base_body = fixture("ingest-valid.json");

    let state1 = AppState::from_db_path(&db_path).expect("state1");
    let router1 = app(state1);
    let first = router1
        .clone()
        .oneshot(json_request("POST", "/v0/ingest", base_body.clone()))
        .await
        .expect("ingest");
    assert_eq!(first.status(), StatusCode::CREATED);

    let state2 = AppState::from_db_path(&db_path).expect("state2");
    let router2 = app(state2);

    let mut duplicate_same: Value = serde_json::from_str(&base_body).expect("json");
    duplicate_same["batch_id"] = Value::String("bat_20260419_0002".to_string());
    let duplicate_same_resp = router2
        .clone()
        .oneshot(json_request(
            "POST",
            "/v0/ingest",
            serde_json::to_string(&duplicate_same).expect("json"),
        ))
        .await
        .expect("duplicate same");
    assert_eq!(duplicate_same_resp.status(), StatusCode::CREATED);
    let duplicate_same_json = response_json(duplicate_same_resp).await;
    assert_eq!(duplicate_same_json["duplicates_ignored"]["runs"], 1);

    let mut duplicate_diff: Value = serde_json::from_str(&base_body).expect("json");
    duplicate_diff["batch_id"] = Value::String("bat_20260419_0003".to_string());
    duplicate_diff["traces"][0]["run"]["ended_at"] =
        Value::String("2026-04-20T02:01:01.500Z".to_string());
    duplicate_diff["traces"][0]["run"]["duration_ms"] = Value::from(1500);
    duplicate_diff["traces"][0]["spans"][0]["ended_at"] =
        Value::String("2026-04-20T02:01:01.500Z".to_string());
    duplicate_diff["traces"][0]["spans"][0]["duration_ms"] = Value::from(1500);
    let duplicate_diff_resp = router2
        .oneshot(json_request(
            "POST",
            "/v0/ingest",
            serde_json::to_string(&duplicate_diff).expect("json"),
        ))
        .await
        .expect("duplicate diff");
    assert_eq!(duplicate_diff_resp.status(), StatusCode::CONFLICT);
    let duplicate_diff_json = response_json(duplicate_diff_resp).await;
    assert_eq!(duplicate_diff_json["code"], "ENTITY_CONFLICT");

    let _ = std::fs::remove_file(db_path);
}
