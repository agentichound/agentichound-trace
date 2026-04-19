# Collector Quickstart (Local Preview)

This is the minimal local flow for manually verifying the frozen collector contract.

## 1) Run the collector

From repo root:

```powershell
cargo run --manifest-path collector/Cargo.toml
```

Default bind:

- `http://127.0.0.1:3000`
- SQLite DB: `collector.db` (current working directory)

Optional port override:

```powershell
$env:PORT="4000"
cargo run --manifest-path collector/Cargo.toml
```

Optional DB path override:

```powershell
$env:COLLECTOR_DB_PATH=".\\tmp\\agentichound-preview.db"
cargo run --manifest-path collector/Cargo.toml
```

## 2) Ingest a valid fixture

```powershell
curl -i -X POST http://127.0.0.1:3000/v0/ingest `
  -H "Content-Type: application/json" `
  --data-binary "@collector/fixtures/ingest-valid.json"
```

Expected:

- HTTP `201`
- JSON includes `accepted=true`, `replayed=false`

## 3) List runs

```powershell
curl -i "http://127.0.0.1:3000/v0/runs?limit=10"
```

Expected:

- HTTP `200`
- `runs` array includes ingested `run_id`

## 4) Fetch run detail

```powershell
curl -i "http://127.0.0.1:3000/v0/runs/run_happy_1"
```

Expected:

- HTTP `200`
- body has `trace` (schema v0 document) and `meta`

## Manual Verification (Replay + Conflict)

Idempotent replay:

```powershell
curl -i -X POST http://127.0.0.1:3000/v0/ingest `
  -H "Content-Type: application/json" `
  --data-binary "@collector/fixtures/ingest-valid.json"
```

Expected second call:

- HTTP `200`
- `replayed=true`

Batch conflict:

- Reuse same `batch_id` with a non-equivalent canonicalized body.
- Expected: HTTP `409`, `code="BATCH_ID_CONFLICT"`.
