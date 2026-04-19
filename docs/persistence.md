# Persistence Note (Phase 1 Current State)

## Current storage behavior

- Collector uses **SQLite local-first persistence**.
- Default database path is `collector.db` in the process working directory.
- Optional override: `COLLECTOR_DB_PATH`.

## Durability guarantees (current)

- Ingest is atomic per request.
- Accepted ingests, batches, run documents, and entity identity records persist across restarts.
- Frozen idempotency and duplicate/conflict semantics are evaluated against persisted SQLite state.

## Scope limits (current)

- Single-process local runtime.
- No multi-tenant partitioning.
- No remote durability or replication.

## SQLite schema guardrail

- Collector stores a schema version in `schema_meta`.
- Startup validates the expected schema version before serving requests.
- Version mismatch fails fast to prevent silent data incompatibility.
