# Vision

## Product

AgenticHound Trace is open-source runtime tracing and bottleneck profiling for agent systems, tool calls, and execution paths.

## Core Positioning

Runtime truth for agent systems.

## User Questions It Must Answer

- Why was this run slow?
- Where did retries start?
- Was the bottleneck the model or the tool?
- What failed?
- What path did the agent take?

## Phase 1 Strategy

- Rust-first implementation grounded in real runtime usage
- TypeScript second with matching contract semantics
- Python and Go deferred until core loop is proven
- Local-first collector and inspection workflow

## Product Discipline

- Trace first, Gateway later
- No platform drift
- No enterprise bloat in Phase 1
- Build for evidence, not feature volume
