# Quick Diagnostic Demo

Use this 30-second flow to prove the first insight surface with a real collector-backed run.

## 1. Start the collector

```powershell
$env:PORT = "3010"
$env:COLLECTOR_DB_PATH = "collector/diagnostics-demo.db"
cargo run --manifest-path collector/Cargo.toml
```

## 2. Ingest a healthy run

```powershell
curl.exe -i -X POST http://127.0.0.1:3010/v0/ingest `
  -H "Content-Type: application/json" `
  --data-binary "@collector/fixtures/ingest-valid.json"
```

## 3. Ingest a low-progress run

```powershell
curl.exe -i -X POST http://127.0.0.1:3010/v0/ingest `
  -H "Content-Type: application/json" `
  --data-binary "@collector/fixtures/ingest-pcd.json"
```

## 4. Diagnose the run

```powershell
cargo run --manifest-path cli/Cargo.toml --bin agentichound -- diagnose `
  --collector-url http://127.0.0.1:3010 `
  --run-id run_pcd_1
```

## 5. Optional JSON output

```powershell
cargo run --manifest-path cli/Cargo.toml --bin agentichound -- diagnose `
  --collector-url http://127.0.0.1:3010 `
  --run-id run_pcd_1 `
  --json
```

## What you should see

- `run_happy_1` should score `low`
- `run_pcd_1` should score `high` or `critical`
- the output should explain the result using only trace-visible signals
