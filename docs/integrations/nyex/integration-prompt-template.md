# NYEX -> AgenticHound Trace Integration Prompt Template

Use this prompt when asking an agent (ChatGPT/NYEX/Codex) to implement or extend NYEX tracing into AgenticHound.

This is a reference integration workflow template for controlled integration work. It is not part of frozen schema or collector contract definitions.

## Prompt

```md
You are working inside NYEX main.

Project: NYEX -> AgenticHound Trace integration
Workspace: E:\Nyex\nyex-main
AgenticHound repo: E:\agentichound
Collector URL: http://127.0.0.1:3000

Primary objective:
Implement the smallest safe integration slice that emits valid AgenticHound trace documents from one real NYEX runtime path, then validate end-to-end with collector APIs.

Non-negotiable constraints:
- Keep blast radius low and changes reversible.
- Do not redesign NYEX architecture.
- Do not modify AgenticHound schema/contracts.
- Do not add broad tracing rollout across all NYEX paths.
- Do not change router/handover/protocol behavior unless explicitly requested.
- Prefer additive adapter/hook changes.
- If blocked, report exact blocker with command output.

Execution rules:
1. Verify current state first:
   - active branch
   - modified files
   - existing NYEX POC hook location
   - AgenticHound SDK dependency path
2. Use one real NYEX execution path only.
3. Validate with collector:
   - GET /v0/runs
   - GET /v0/runs/{run_id}
4. Return exact evidence (commands + outputs + run_id).
5. Keep output implementation-first and low-noise.

Expected mapping quality:
- Produce valid run/span IDs according to schema regex.
- Emit schema_version expected by collector.
- Include run status/timestamps/duration.
- Include spans for orchestration and mapped model/tool where available.
- Include usage tokens when runtime trace data is available.

If a code change is required:
- Keep it minimal and isolated to NYEX integration adapter/hook.
- Explain exactly why the change is required.
- Add/adjust targeted tests only.

Validation sequence (must execute):
1. Start collector.
2. Run NYEX traced command with:
   - AGENTICHOUND_TRACE=1
   - AGENTICHOUND_COLLECTOR_URL=http://127.0.0.1:3000
3. Query /v0/runs.
4. Capture emitted run_id.
5. Query /v0/runs/{run_id}.
6. Confirm trace contains required fields.

Legacy compatibility note:
- NYEX-prefixed vars (`NYEX_AGENTICHOUND_TRACE`, `NYEX_AGENTICHOUND_COLLECTOR_URL`) are temporary fallback only.

Output format (strict):
1. Current integration state
2. Commands run
3. Auth/dev path used
4. Collector validation
5. NYEX execution result
6. AgenticHound trace evidence
7. Final verdict (success/blocker + classification)
```

## Optional extension block (for richer telemetry slices)

Add this only when you explicitly want deeper tracing:

```md
Enhancement scope:
- Improve span richness without changing runtime decisions.
- Include token usage and cost-ready fields where available.
- Keep telemetry-only semantics.

Additional acceptance criteria:
- At least one model span includes token usage when routing usage exists.
- Tool spans include duration/outcome.
- Trace detail remains schema-valid.
```

## Notes for planners

- NYEX trace source of truth is orchestration trace (`OrchestrationTraceRecord`).
- Prefer mapping from existing fields (routing decisions, attempts, tool invocations, validator result).
- Do not fabricate semantic fields that are not present in runtime trace.
