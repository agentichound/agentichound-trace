# Collector Contract v0 (Frozen)

This contract is Phase 1 only and aligned to `schemas/trace.schema.json` (`schema_version = "v0"`).

## Scope and Non-Scope

In scope:

- Local HTTP ingestion and read APIs
- Deterministic idempotency and replay behavior
- Strict validation and conflict handling

Out of scope:

- Auth
- Multi-tenant behavior
- Streaming ingest
- Advanced filtering
- Enterprise retention/governance features

## Transport

- Protocol: HTTP/1.1 or HTTP/2
- Content-Type for request bodies: `application/json`
- Character set: UTF-8

## Endpoint Surface (Frozen)

- `POST /v0/ingest`
- `GET /v0/runs`
- `GET /v0/runs/{run_id}`

No other Phase 1 endpoints are part of this contract.

## Common Conventions

- Timestamps: RFC3339 UTC date-time string
- `schema_version`: must be `"v0"`
- IDs are immutable once accepted
- Ingest writes are atomic per request
  - Any validation/conflict error means no writes from that request are committed

## Ingestion Limits (Frozen)

- Max request payload size: `524288` bytes (512 KiB)
- Max traces per ingest request (`traces.length`): `32`
- Max total entities per request:
  - `sum(1 + spans + events + errors + usage over all traces) <= 5000`
- Per trace limits:
  - `spans <= 1000`
  - `events <= 2000`
  - `errors <= 200`
  - `usage <= 1000`

Requests exceeding limits return `413` (payload size) or `422` (entity limits).

## POST /v0/ingest

### Request Body (exact envelope)

```json
{
  "batch_id": "bat_20260419_0001",
  "sent_at": "2026-04-19T19:00:00.000Z",
  "traces": [
    {
      "schema_version": "v0",
      "run": {},
      "spans": [],
      "events": [],
      "errors": [],
      "usage": []
    }
  ]
}
```

Required fields:

- `batch_id` (string, pattern `^bat_[A-Za-z0-9_-]+$`)
- `sent_at` (RFC3339 UTC date-time)
- `traces` (array, min 1, max 32)

Each trace in `traces` must validate against `schemas/trace.schema.json`.

### Validation Rules

For each trace:

- JSON Schema validation must pass
- Referential integrity must pass:
  - all `span.run_id`, `event.run_id`, `error.run_id`, `usage.run_id` equal `run.run_id`
  - all referenced span IDs exist
  - retry span rules are respected
  - `duration_ms` matches timestamp delta
  - `usage.total_tokens == prompt_tokens + completion_tokens`

For request:

- envelope fields and limits must pass
- content-type must be `application/json`

### Idempotency and Replay (Frozen)

Idempotency key: `batch_id`.

Collector behavior:

1. First-seen `batch_id`:
   - validate and ingest atomically
   - return `201 Created`
2. Replay with same `batch_id` and canonicalized-equivalent JSON body:
   - no new writes
   - return `200 OK` with `replayed = true`
3. Replay with same `batch_id` and non-equivalent canonicalized JSON body:
   - no writes
   - return `409 Conflict` (`code = "BATCH_ID_CONFLICT"`)

Duplicate entity handling across different `batch_id` values:

- Same entity ID + identical payload: ignored (counted in `duplicates_ignored`)
- Same entity ID + different payload: `409 Conflict` (`code = "ENTITY_CONFLICT"`), atomic reject

### Success Response Shape

For `201 Created` (new ingest):

```json
{
  "accepted": true,
  "replayed": false,
  "ingestion_id": "ing_9d2f8b5d",
  "batch_id": "bat_20260419_0001",
  "received": {
    "traces": 1,
    "runs": 1,
    "spans": 3,
    "events": 3,
    "errors": 0,
    "usage": 2
  },
  "inserted": {
    "runs": 1,
    "spans": 3,
    "events": 3,
    "errors": 0,
    "usage": 2
  },
  "duplicates_ignored": {
    "runs": 0,
    "spans": 0,
    "events": 0,
    "errors": 0,
    "usage": 0
  }
}
```

For `200 OK` replay:

- Same shape
- `accepted = true`
- `replayed = true`
- `inserted.* = 0`

### Error Response Shape (all error statuses)

```json
{
  "accepted": false,
  "code": "VALIDATION_ERROR",
  "message": "trace[0].run.run_id is required",
  "details": [
    {
      "path": "traces[0].run.run_id",
      "reason": "required"
    }
  ]
}
```

## GET /v0/runs

Returns recent run summaries.

### Query Parameters

- `limit` (optional integer, default `50`, min `1`, max `100`)
- `cursor` (optional opaque string from prior response)

No other filters are part of v0.

### Response

```json
{
  "runs": [
    {
      "run_id": "run_happy_1",
      "status": "ok",
      "started_at": "2026-04-19T18:01:00.000Z",
      "ended_at": "2026-04-19T18:01:01.400Z",
      "duration_ms": 1400,
      "span_count": 3,
      "event_count": 3,
      "error_count": 0,
      "usage_count": 2,
      "total_tokens": 200,
      "estimated_cost_usd": 0.0028,
      "last_ingested_at": "2026-04-19T19:00:00.000Z"
    }
  ],
  "next_cursor": "c_00000051"
}
```

Ordering: `started_at` descending, then `run_id` ascending as tie-breaker.

## GET /v0/runs/{run_id}

Returns one canonical trace document for `run_id`.

### Response (200)

```json
{
  "trace": {
    "schema_version": "v0",
    "run": {},
    "spans": [],
    "events": [],
    "errors": [],
    "usage": []
  },
  "meta": {
    "run_id": "run_happy_1",
    "ingested_at": "2026-04-19T19:00:00.000Z"
  }
}
```

- `trace` must match `schemas/trace.schema.json`.
- `meta` is collector metadata and not part of trace schema.

### Not Found (404)

```json
{
  "accepted": false,
  "code": "RUN_NOT_FOUND",
  "message": "run_id not found"
}
```

## Status Codes (Frozen)

`POST /v0/ingest`:

- `201` created
- `200` idempotent replay (same `batch_id` + canonicalized-equivalent body)
- `400` malformed JSON
- `409` batch/entity conflict
- `413` payload too large
- `415` unsupported content-type
- `422` semantic validation failure
- `500` internal collector error

`GET /v0/runs`:

- `200` success
- `400` invalid query parameters
- `500` internal collector error

`GET /v0/runs/{run_id}`:

- `200` success
- `404` run not found
- `500` internal collector error
