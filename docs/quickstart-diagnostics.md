# Quick Diagnostic Demo

Use this browser-first flow to watch runs and diagnostics appear live from a local collector.

## 1. Start the collector service

```powershell
$env:PORT = "3010"
$env:COLLECTOR_DB_PATH = "collector/diagnostics-demo.db"
cargo run --manifest-path collector/Cargo.toml
```

## 2. Open the local viewer

Open:

```text
http://127.0.0.1:3010/viewer
```

The page auto-polls every 2.5 seconds and supports pause/resume.

## 3. Ingest a healthy run

```powershell
curl.exe -i -X POST http://127.0.0.1:3010/v0/ingest `
  -H "Content-Type: application/json" `
  --data-binary "@collector/fixtures/ingest-valid.json"
```

## 4. Ingest a low-progress run

```powershell
curl.exe -i -X POST http://127.0.0.1:3010/v0/ingest `
  -H "Content-Type: application/json" `
  --data-binary "@collector/fixtures/ingest-pcd.json"
```

## 5. Verify in the viewer

- recent runs should appear in the left list
- selecting a run shows severity, summary, reasons, and supporting signals
- healthy fixture should show `low`
- pcd fixture should show `high` or `critical`
- pause/resume should stop/restart polling updates

## 6. Optional CLI verification (secondary path)

```powershell
cargo run --manifest-path cli/Cargo.toml --bin agentichound -- diagnose `
  --collector-url http://127.0.0.1:3010 `
  --run-id run_pcd_1
```

## 7. Optional JSON output

```powershell
cargo run --manifest-path cli/Cargo.toml --bin agentichound -- diagnose `
  --collector-url http://127.0.0.1:3010 `
  --run-id run_pcd_1 `
  --json
```

## What you should see

- `run_happy_1` should score `low`
- `run_pcd_1` should score `high` or `critical`
- diagnostics should explain results using only trace-visible signals
