# Roadmap

## Current Baseline (Completed)

- Canonical repository structure is locked
- Trace schema v0 is frozen and validated
- Collector contract v0 is frozen
- Rust collector implements:
  - `POST /v0/ingest`
  - `GET /v0/runs`
  - `GET /v0/runs/{run_id}`
- SQLite local-first persistence is implemented and restart-tested
- Contract fixtures and validation scripts are in place

## Near-Term (Public Preview Hardening)

- Add SQLite schema migration/version guardrails
- Tighten docs and release notes for public preview
- Keep API semantics stable while improving implementation quality

## Next Build Phase

- Expand Rust SDK beyond skeleton API surface
- Keep TypeScript SDK in parity with frozen schema/contract
- Add CLI ergonomics for inspect/export flows

## Deferred by Design

- Auth and multi-tenant behavior
- Enterprise governance features
- Streaming ingestion
- Reference integration: NYEX
- Gateway/control-plane features
