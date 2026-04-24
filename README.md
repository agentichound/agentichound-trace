# AgenticHound Trace

Open-source runtime tracing and bottleneck profiling for agent systems, tool calls, and execution paths.

AgenticHound Trace is runtime truth and diagnostics for agent systems.

It exists to show where agents lose time, cost, and reliability across model calls, tool calls, retries, failures, orchestration steps, and execution bottlenecks.

## Quick Diagnostic Demo

Run a collector, ingest a trace, then diagnose it:

```powershell
cargo run --manifest-path collector/Cargo.toml
curl -X POST http://127.0.0.1:3000/v0/ingest -H "Content-Type: application/json" --data-binary "@collector/fixtures/ingest-pcd.json"
cargo run --manifest-path cli/Cargo.toml --bin agentichound -- diagnose --collector-url http://127.0.0.1:3000 --run-id run_pcd_1
```

Example output:

```text
severity: critical
summary: Run strongly resembles progress collapse with repeated work and limited progress.
```

Full flow: [docs/quickstart-diagnostics.md](docs/quickstart-diagnostics.md)

## Why this is different from a generic trace tree

A generic trace tree shows what happened.

AgenticHound shows when a run is stuck, looping, retry-heavy, or making little progress despite lots of activity.

Generic trace:

`9 spans executed`

AgenticHound:

`critical: retry-heavy run with limited forward progress`

This is runtime truth first: the diagnostic only uses fields already present in the trace.

## Phase 1 Priority

1. Rust
2. TypeScript / Node
3. Python later
4. Go later

## Canonical Repository Layout

- `docs/`
- `schemas/`
- `sdk/rust/`
- `sdk/typescript/`
- `sdk/python/` (planned)
- `sdk/go/` (planned)
- `collector/`
- `viewer/`
- `cli/`
- `examples/`
- `scripts/`

## Status

Early technical preview.

What is stable now:

- Canonical repo layout
- Frozen trace schema v0 (`schemas/trace.schema.json`)
- Frozen collector contract v0 (`docs/collector-contract.md`)
- Runnable Rust collector (`POST /v0/ingest`, `GET /v0/runs`, `GET /v0/runs/{run_id}`)
- SQLite local-first persistence with restart durability
- Contract and persistence test coverage for idempotency/duplicate/conflict behavior

What is not in scope yet:

- Auth, multi-tenant behavior, streaming ingest
- UI
- Reference integrations beyond the documented example

## Non-goals (Phase 1)

- Agent framework
- Workflow engine
- Enterprise control plane
- Generic observability suite
- Heavy OpenTelemetry replacement
- Multi-tenant SaaS

## Product Rule

Trace first, Gateway later.

## Quickstart

Run collector locally:

```powershell
cargo run --manifest-path collector/Cargo.toml
```

Ingest a fixture:

```powershell
curl -i -X POST http://127.0.0.1:3000/v0/ingest `
  -H "Content-Type: application/json" `
  --data-binary "@collector/fixtures/ingest-valid.json"
```

List runs:

```powershell
curl -i "http://127.0.0.1:3000/v0/runs?limit=10"
```

Get run detail:

```powershell
curl -i "http://127.0.0.1:3000/v0/runs/run_happy_1"
```

More detail:

- [Collector quickstart](docs/collector-quickstart.md)
- [Quick diagnostic demo](docs/quickstart-diagnostics.md)
- [Persistence](docs/persistence.md)
- [Trace schema v0](docs/trace-schema-v0.md)
- [Collector contract v0](docs/collector-contract.md)

## Integration templates

These templates guide controlled integration work and are not part of frozen schema/contract definitions.

Hard rule for integrations: preserve runtime truth. Do not invent semantic fields that are not present in the source runtime trace.

- [Integration guide](docs/integrations/README.md)
- [NYEX reference integration prompt template](docs/integrations/nyex/integration-prompt-template.md)
