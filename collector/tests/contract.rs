use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use collector_core::{app, types::IngestSuccessResponse, AppState};
use serde_json::Value;
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

#[tokio::test]
async fn fixture_decode_and_validation_passes_for_valid_fixture() {
    let state = AppState::in_memory();
    let router = app(state);
    let body = fixture("ingest-valid.json");

    let resp = router
        .oneshot(json_request("POST", "/v0/ingest", body))
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn missing_batch_id_maps_to_validation_error() {
    let state = AppState::in_memory();
    let router = app(state);
    let body = fixture("ingest-invalid.json");

    let resp = router
        .oneshot(json_request("POST", "/v0/ingest", body))
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = response_json(resp).await;
    assert_eq!(json["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn schema_invalid_trace_maps_to_validation_error() {
    let state = AppState::in_memory();
    let router = app(state);
    let bad = r#"{
      "batch_id":"bat_schema_bad_1",
      "sent_at":"2026-04-19T19:00:00.000Z",
      "traces":[
        {
          "schema_version":"v0",
          "run":{"run_id":"run_bad_1","started_at":"2026-04-19T19:00:00.000Z","ended_at":"2026-04-19T19:00:01.000Z","duration_ms":1000,"status":"ok"},
          "spans":[],
          "events":[],
          "errors":[],
          "usage":[]
        }
      ]
    }"#;

    let resp = router
        .oneshot(json_request("POST", "/v0/ingest", bad.to_string()))
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = response_json(resp).await;
    assert_eq!(json["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn same_batch_id_identical_body_is_idempotent_replay() {
    let state = AppState::in_memory();
    let router = app(state);
    let body = fixture("ingest-valid.json");

    let first = router
        .clone()
        .oneshot(json_request("POST", "/v0/ingest", body.clone()))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);

    let second = router
        .oneshot(json_request("POST", "/v0/ingest", body))
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let json = response_json(second).await;
    assert_eq!(json["replayed"], true);
}

#[tokio::test]
async fn same_batch_id_different_body_is_batch_id_conflict() {
    let state = AppState::in_memory();
    let router = app(state);
    let body = fixture("ingest-valid.json");

    let first = router
        .clone()
        .oneshot(json_request("POST", "/v0/ingest", body.clone()))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);

    let mut as_json: Value = serde_json::from_str(&body).expect("valid fixture json");
    as_json["sent_at"] = Value::String("2026-04-19T19:00:01.000Z".to_string());
    let changed = serde_json::to_string(&as_json).expect("json");

    let second = router
        .oneshot(json_request("POST", "/v0/ingest", changed))
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::CONFLICT);
    let json = response_json(second).await;
    assert_eq!(json["code"], "BATCH_ID_CONFLICT");
}

#[tokio::test]
async fn duplicate_entity_same_payload_across_batches_is_ignored() {
    let state = AppState::in_memory();
    let router = app(state);
    let body = fixture("ingest-valid.json");
    let first = router
        .clone()
        .oneshot(json_request("POST", "/v0/ingest", body.clone()))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);

    let mut second_json: Value = serde_json::from_str(&body).expect("valid json");
    second_json["batch_id"] = Value::String("bat_20260419_0002".to_string());
    let second_body = serde_json::to_string(&second_json).expect("json");

    let second = router
        .oneshot(json_request("POST", "/v0/ingest", second_body))
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::CREATED);
    let bytes = to_bytes(second.into_body(), usize::MAX)
        .await
        .expect("body");
    let parsed: IngestSuccessResponse = serde_json::from_slice(&bytes).expect("success response");
    assert_eq!(parsed.duplicates_ignored.runs, 1);
    assert_eq!(parsed.inserted.runs, 0);
}

#[tokio::test]
async fn duplicate_entity_different_payload_across_batches_is_entity_conflict() {
    let state = AppState::in_memory();
    let router = app(state);
    let body = fixture("ingest-valid.json");
    let first = router
        .clone()
        .oneshot(json_request("POST", "/v0/ingest", body.clone()))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);

    let mut second_json: Value = serde_json::from_str(&body).expect("valid json");
    second_json["batch_id"] = Value::String("bat_20260419_0003".to_string());
    second_json["traces"][0]["spans"][0]["duration_ms"] = Value::from(1500);
    second_json["traces"][0]["run"]["duration_ms"] = Value::from(1500);
    second_json["traces"][0]["run"]["ended_at"] =
        Value::String("2026-04-20T02:01:01.500Z".to_string());
    second_json["traces"][0]["spans"][0]["ended_at"] =
        Value::String("2026-04-20T02:01:01.500Z".to_string());
    let second_body = serde_json::to_string(&second_json).expect("json");

    let second = router
        .oneshot(json_request("POST", "/v0/ingest", second_body))
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::CONFLICT);
    let json = response_json(second).await;
    assert_eq!(json["code"], "ENTITY_CONFLICT");
}

#[tokio::test]
async fn viewer_route_serves_html_page() {
    let state = AppState::in_memory();
    let router = app(state);
    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/viewer")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("viewer response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    let html = String::from_utf8(bytes.to_vec()).expect("utf8 html");
    assert!(html.contains("AgenticHound Trace"));
    assert!(html.contains("Local Runtime Diagnostics"));
}

#[tokio::test]
async fn run_diagnostics_endpoint_returns_progress_collapse_output() {
    let state = AppState::in_memory();
    let router = app(state);
    let body = fixture("ingest-pcd.json");

    let ingest = router
        .clone()
        .oneshot(json_request("POST", "/v0/ingest", body))
        .await
        .expect("ingest");
    assert_eq!(ingest.status(), StatusCode::CREATED);

    let diagnostics = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v0/runs/run_pcd_1/diagnostics")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("diagnostics");
    assert_eq!(diagnostics.status(), StatusCode::OK);
    let json = response_json(diagnostics).await;
    assert_eq!(json["run_id"], "run_pcd_1");
    assert_eq!(
        json["diagnostics"][0]["diagnostic"],
        "progress_collapse_detector"
    );
    assert!(
        json["diagnostics"][0]["severity"] == "high"
            || json["diagnostics"][0]["severity"] == "critical"
    );
}
