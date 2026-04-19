AgenticHound Trace v0.1.0-alpha.1 is the first technical preview.

What is included:
- Frozen trace schema v0 (`schemas/trace.schema.json`)
- Frozen collector contract v0 (`docs/collector-contract.md`)
- Runnable Rust collector with:
  - `POST /v0/ingest`
  - `GET /v0/runs`
  - `GET /v0/runs/{run_id}`
- SQLite local-first persistence with restart durability
- Contract fixtures and validation scripts

What this release is for:
- Verify contract fidelity and deterministic ingest behavior
- Validate replay/conflict semantics under real retries
- Provide a stable base for next Phase 1 collector hardening

Out of scope in this preview:
- Auth, multi-tenant behavior, streaming ingest
- UI
- NYEX integration

Phase 1 rule remains unchanged: Trace first, Gateway later.