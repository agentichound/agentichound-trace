# Architecture

## Goals

- Capture agent runtime execution with low instrumentation overhead
- Preserve execution causality across runs, spans, events, and retries
- Keep data model and contract stable enough to build SDK and collector in parallel

## System Shape

1. SDK inside app/runtime emits trace entities
2. Collector ingests and validates payload batches
3. Collector stores normalized records (SQLite first)
4. CLI/viewer surfaces run path, latency bottlenecks, retries, failures, and cost

## Canonical Components

### SDKs (`sdk/`)

- `sdk/rust/`: first-class Phase 1 SDK
- `sdk/typescript/`: second SDK with contract parity
- `sdk/python/`: deferred placeholder
- `sdk/go/`: deferred placeholder

### Collector (`collector/`)

- Ingest contract validation
- Local persistence (SQLite first)
- Query primitives for run list/detail and summaries

### Inspection Surfaces (`cli/`, `viewer/`)

- Run listing
- Run detail timeline/path
- Bottleneck and retry inspection
- Export path for structured trace artifacts

### Schema (`schemas/`)

- `schemas/trace.schema.json`
- `schemas/examples/`

## Boundaries

- Not an orchestration engine
- Not a control plane
- Not an enterprise governance layer
- Not a broad APM replacement
