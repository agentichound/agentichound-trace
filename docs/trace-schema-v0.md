# Trace Schema v0 (Frozen)

This document is the exact contract for `schemas/trace.schema.json`.

## Top-Level Envelope

Required fields:

- `schema_version` (`"v0"`)
- `run` (object)
- `spans` (array, min 1)
- `events` (array)
- `errors` (array)
- `usage` (array)

No additional top-level fields are allowed.

## Entity: `run`

Required:

- `run_id` (`^run_[A-Za-z0-9_-]+$`)
- `started_at` (RFC3339 UTC date-time string)
- `ended_at` (RFC3339 UTC date-time string)
- `duration_ms` (integer, >= 0)
- `status` (`ok | error | cancelled`)

## Entity: `span`

Required:

- `span_id` (`^spn_[A-Za-z0-9_-]+$`)
- `run_id` (`^run_[A-Za-z0-9_-]+$`)
- `parent_span_id` (`span_id` or `null`)
- `retry_of_span_id` (`span_id` or `null`)
- `kind` (`model | tool | orchestration | retry | handoff | approval`)
- `name` (1..120 chars)
- `started_at` (RFC3339 UTC date-time string)
- `ended_at` (RFC3339 UTC date-time string)
- `duration_ms` (integer, >= 0)
- `status` (`ok | error | cancelled`)

Rule:

- If `kind == "retry"`, `retry_of_span_id` must be non-null span id.
- If `kind != "retry"`, `retry_of_span_id` must be null.

## Entity: `event`

Required:

- `event_id` (`^evt_[A-Za-z0-9_-]+$`)
- `run_id` (`^run_[A-Za-z0-9_-]+$`)
- `span_id` (`^spn_[A-Za-z0-9_-]+$`)
- `kind` (`model | tool | orchestration | retry | handoff | approval`)
- `ts` (RFC3339 UTC date-time string)
- `payload` (object)

## Entity: `error`

Required:

- `error_id` (`^err_[A-Za-z0-9_-]+$`)
- `run_id` (`^run_[A-Za-z0-9_-]+$`)
- `span_id` (`^spn_[A-Za-z0-9_-]+$`)
- `ts` (RFC3339 UTC date-time string)
- `code` (1..80 chars)
- `message` (1..500 chars)
- `retryable` (boolean)

## Entity: `usage`

Required:

- `usage_id` (`^use_[A-Za-z0-9_-]+$`)
- `run_id` (`^run_[A-Za-z0-9_-]+$`)
- `span_id` (`^spn_[A-Za-z0-9_-]+$`)
- `kind` (`model | tool`)
- `prompt_tokens` (integer, >= 0)
- `completion_tokens` (integer, >= 0)
- `total_tokens` (integer, >= 0)
- `estimated_cost_usd` (number, >= 0)
- `currency` (`"USD"`)

## Parent-Child and Referential Rules

JSON Schema enforces field shape and enum constraints. Additional integrity checks are required:

- Every `span.run_id`, `event.run_id`, `error.run_id`, and `usage.run_id` must equal `run.run_id`.
- Every non-null `parent_span_id` and `retry_of_span_id` must reference an existing `span_id`.
- Every `event.span_id`, `error.span_id`, and `usage.span_id` must reference an existing `span_id`.
- `duration_ms` values must match `ended_at - started_at` (milliseconds).
- `usage.total_tokens` must equal `prompt_tokens + completion_tokens`.

These checks are enforced by `scripts/validate_trace_samples.py`.

## Sample Trace Set

`schemas/examples/` includes:

- 9 happy path traces (`happy-path-*.json`)
- 9 tool bottleneck traces (`tool-bottleneck-*.json`)
- 9 retry then success traces (`retry-then-success-*.json`)
- 8 failed run traces (`failed-run-*.json`)

Total valid traces: 35.

Invalid sample:

- `invalid-missing-run-id.json`

Why invalid:

- `run.run_id` is missing (required field).
