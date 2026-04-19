# Integration Guide

AgenticHound Trace is generic runtime truth and diagnostics for agent systems.

Use this folder for implementation guidance that helps any agent runtime emit traces that map cleanly to schema v0.

## How integrations work

- Capture real runtime events from the agent system.
- Map those events into the frozen trace schema v0 without inventing semantics.
- Keep the trace aligned to the runtime source of truth.
- Validate against the collector contract and sample traces before broad rollout.

Hard rule for all integrations:

- Do not fabricate semantic fields that are not present in runtime trace.

## Reference integrations

- [NYEX reference integration prompt template](nyex/integration-prompt-template.md)
