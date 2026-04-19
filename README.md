# AgenticHound Trace

Open-source runtime tracing and bottleneck profiling for agent systems, tool calls, and execution paths.

AgenticHound Trace helps developers and AI platform teams see exactly where agents lose time, cost, and reliability. It captures end-to-end execution across model calls, tool calls, retries, failures, and orchestration steps so teams can debug faster and optimize what actually matters.

## Why

Modern agent systems are often bottlenecked less by inference and more by the surrounding runtime: tool calls, auth flows, retries, integrations, and orchestration overhead. AgenticHound Trace makes those bottlenecks visible with evidence.

## Initial scope

- Rust SDK
- Python SDK
- TypeScript / Node SDK
- Minimal local collector
- Local trace inspection
- Structured trace export

## Non-goals

- not an agent framework
- not a workflow engine
- not an enterprise control plane
- not a generic observability suite
