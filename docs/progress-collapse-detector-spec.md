# Progress Collapse Detector (PCD) Specification

## 1. Purpose

Progress Collapse Detector (PCD) identifies runs that consume substantial runtime activity but show weak forward progress.

Its purpose is to answer a narrow operator question:

**Did this run keep doing work, or did it mostly spin?**

PCD matters because agent systems can generate many spans, retries, and errors while still producing little useful advancement. That behavior burns time, tokens, and operator attention even when the run eventually ends.

PCD is a diagnostic. It does not judge semantic correctness of the final answer.

## 2. Scope and Constraints

PCD operates on a single frozen v0 trace document.

PCD must use only signals that already exist in the current or near-term trace model:

- `run`
- `spans`
- `errors`
- `usage`

PCD must not:

- invent task intent
- infer hidden business semantics
- require a UI
- require schema changes
- require collector contract changes

## 3. Definition of Progress

For PCD, progress means observable forward movement through distinct execution steps in the runtime trace.

Because the trace schema does not carry task intent or semantic completion markers, progress must be approximated from trace structure only.

PCD treats the following as progress evidence:

- a non-retry span completes successfully (`status = ok`)
- the run advances through distinct successful non-retry span groups over time
- the run reaches a terminal `run.status` value with limited repetition pressure

PCD treats the following as activity without progress:

- retry spans that repeat prior work
- repeated spans with the same `kind` and `name`
- error-heavy spans that do not introduce new successful span groups
- long runtime with low diversity of successful non-retry spans

Operationally:

- progress evidence is counted from successful non-retry spans
- activity evidence is counted from all spans, retry spans, errors, and usage

Retry spans never count as progress.

## 4. Input Signals

PCD may use the following fields from the frozen trace schema:

### Run fields

- `run.run_id`
- `run.status`
- `run.started_at`
- `run.ended_at`
- `run.duration_ms`

### Span fields

- `span.span_id`
- `span.run_id`
- `span.parent_span_id`
- `span.retry_of_span_id`
- `span.kind`
- `span.name`
- `span.started_at`
- `span.ended_at`
- `span.duration_ms`
- `span.status`

### Error fields

- `error.error_id`
- `error.run_id`
- `error.span_id`
- `error.ts`
- `error.code`
- `error.message`
- `error.retryable`

### Usage fields

- `usage.usage_id`
- `usage.run_id`
- `usage.span_id`
- `usage.kind`
- `usage.prompt_tokens`
- `usage.completion_tokens`
- `usage.total_tokens`
- `usage.estimated_cost_usd`
- `usage.currency`

### Derived signals

PCD may derive the following from the above fields:

- total span count
- non-retry span count
- retry span count
- error count
- usage count
- unique successful span-group count
- repeated span-group count
- repeated span ratio
- retry ratio
- error ratio
- token total
- cost total
- span clusters by `kind + name`

## 5. Detection Logic

PCD should evaluate a run in this order:

### Step 1: Build the run summary

Compute:

- `duration_ms`
- `total_spans`
- `non_retry_spans`
- `retry_spans`
- `error_count`
- `usage_count`
- `successful_non_retry_spans`
- `unique_successful_step_count`
- `repeated_successful_step_count`
- `repeated_span_ratio`
- `retry_ratio`
- `error_ratio`
- `total_tokens` if usage exists
- `total_cost_usd` if usage exists

### Step 2: Identify high activity

A run has high activity if any of the following are true:

- `duration_ms >= 10_000`
- `total_spans >= 10`
- `retry_spans >= 2`
- `error_count >= 1`
- `repeated_successful_step_count >= 2`
- `total_tokens >= 2_000` when usage exists

### Step 3: Identify low progress

A run has low progress if any of the following are true:

- `unique_successful_step_count <= 2` and `total_spans >= 8`
- `repeated_span_ratio >= 0.40`
- `retry_ratio >= 0.20`
- `error_count >= 2`
- `duration_ms >= 30_000` and `unique_successful_step_count <= 3`

### Step 4: Combine the signals

PCD should emit a diagnostic only when both conditions hold:

- high activity is present
- low progress is present

If only one condition is present, the run may be noted as suspicious later, but it should not be classified as a progress collapse.

### Step 5: Strengthen or weaken the result

Severity should increase when collapse signals cluster together:

- high duration plus repeated span clusters
- retry-heavy execution plus errors
- large token spend plus low unique successful step count
- terminal failure or cancellation plus weak progress evidence

Severity should decrease when the run shows clear forward movement:

- several distinct successful non-retry spans
- low repetition pressure
- little retry activity
- successful terminal completion with diverse span groups

## 6. Heuristics and Thresholds

Initial thresholds are intentionally conservative.

### Activity thresholds

- `duration_ms >= 10_000` is meaningful activity
- `total_spans >= 10` is meaningful activity
- `retry_spans >= 2` is retry pressure
- `error_count >= 1` is failure pressure
- `total_tokens >= 2_000` is meaningful token activity when usage exists

### Collapse thresholds

- `unique_successful_step_count <= 2` is weak progress for a busy run
- `repeated_span_ratio >= 0.40` indicates repeated work
- `retry_ratio >= 0.20` indicates retry-dominated execution
- `duration_ms >= 30_000` with few unique successful steps indicates long-running spin
- `error_count >= 2` indicates an error-concentrated run

### Repeated pattern definition

A repeated pattern is a cluster of spans with the same `kind` and `name`.

Retry spans are always repeated-work signals.

Non-retry spans with the same `kind` and `name` are repeated-work signals when they recur within the same run.

## 7. Severity Levels

PCD should use four severity levels.

### Low

The run shows some repetition or retry pressure, but the evidence for collapse is weak.

Typical shape:

- activity is above baseline
- progress evidence exists
- repetition pressure is visible but limited

### Medium

The run is clearly busy and the progress signal is weak enough that an operator should inspect it.

Typical shape:

- at least one strong collapse threshold is met
- the run has notable repetition or retry pressure
- successful step diversity is limited

### High

The run is strongly collapse-like.

Typical shape:

- repeated work dominates the run
- retries or errors are concentrated
- long runtime produces little new successful step diversity

### Critical

The run is overwhelmingly collapse-like and likely wasted substantial time or budget.

Typical shape:

- very high repetition pressure
- sustained runtime with little successful step diversity
- multiple retries or errors
- terminal failure or cancellation, or no clear recovery after repeated work

## 8. Output Schema

PCD output is a logical diagnostic result, not a schema change to the trace contract.

A result should include:

- `diagnostic`: fixed value `progress_collapse_detector`
- `run_id`
- `severity`
- `summary`
- `reasons`
- `supporting_signals`
- `confidence` if the implementation chooses to expose one

### `reasons`

An ordered list of machine-readable reasons.

Each reason should include:

- `code`
- `message`
- `signal_refs`

### `supporting_signals`

A structured block with the measured values used by the decision.

Recommended fields:

- `duration_ms`
- `total_spans`
- `non_retry_spans`
- `retry_spans`
- `error_count`
- `usage_count`
- `unique_successful_step_count`
- `repeated_successful_step_count`
- `repeated_span_ratio`
- `retry_ratio`
- `total_tokens`
- `total_cost_usd`

### `summary`

A single operator-facing sentence that states the likely outcome in plain language.

## 9. Example Output

### CLI-style summary

```text
[AgenticHound Diagnostic] Progress Collapse Detector
run_id: run_019da6eb-cc4e-7913-ad3d-3d84bc7e6c7f
severity: medium
status: ok
duration_ms: 18420
spans: total=14 non_retry=12 retries=2 errors=1
usage: total_tokens=5329 estimated_cost_usd=0.00

signals:
- repeated span cluster: model.planner/openai:gpt-5.3-codex (4x)
- retry concentration: 2 retry spans in the same run
- low unique-successful-step count relative to total activity

diagnostic:
"Run appears activity-heavy with weak forward progress."
```

### JSON-style diagnostic output

```json
{
  "diagnostic": "progress_collapse_detector",
  "run_id": "run_019da6eb-cc4e-7913-ad3d-3d84bc7e6c7f",
  "severity": "medium",
  "summary": "Run appears activity-heavy with weak forward progress.",
  "reasons": [
    {
      "code": "repeated_span_cluster",
      "message": "The run contains a repeated model span cluster.",
      "signal_refs": ["spans.kind", "spans.name"]
    },
    {
      "code": "retry_pressure",
      "message": "Retry activity is concentrated relative to total span count.",
      "signal_refs": ["spans.kind", "spans.retry_of_span_id"]
    }
  ],
  "supporting_signals": {
    "duration_ms": 18420,
    "total_spans": 14,
    "non_retry_spans": 12,
    "retry_spans": 2,
    "error_count": 1,
    "usage_count": 3,
    "unique_successful_step_count": 3,
    "repeated_successful_step_count": 4,
    "repeated_span_ratio": 0.33,
    "retry_ratio": 0.14,
    "total_tokens": 5329,
    "total_cost_usd": 0.0
  }
}
```

## 10. False Positives and Limitations

PCD can misclassify runs in several cases:

- A legitimate multi-step workflow can look repetitive because it must revisit the same tool or model step.
- A short but successful run can still have a high retry ratio if the task is naturally retry-prone.
- A long run with sparse spans can look collapsed even if work is happening outside the traced surface.
- If usage is missing, cost and token-based pressure cannot be used.
- If span naming is poor, repeated clusters become less reliable.

PCD also has a role-level limitation:

- current trace data does not reliably distinguish semantic agent roles
- PCD can only evaluate the structure of the emitted trace, not which actor intended each step

PCD does not claim semantic correctness, only collapse likelihood.

## 11. Validation Approach

PCD should be validated against real trace samples and edge cases.

### Sample traces

Use the frozen sample set and verify that PCD behaves consistently on:

- happy path traces
- tool bottleneck traces
- retry-then-success traces
- failed run traces

Expected pattern:

- happy path should usually score low or none
- tool bottleneck runs should often score medium
- retry-then-success runs should often score medium or high depending on repetition
- failed runs with repeated work should often score high or critical

### Reference integration traces

Validate against traced runs from the reference integration once it emits real runtime data.

The goal is to ensure the detector works on actual operational traces, not synthetic examples only.

### Edge cases

Check at least these cases:

- run with no usage data
- run with one short error and no retries
- run with many spans but good forward progress
- run with repeated span names but a clear successful finish
- run with mostly retry spans and little else

## 12. Non-Goals

PCD does not:

- prove semantic task success
- replace human judgment
- infer hidden agent intent
- diagnose root cause beyond trace-visible structure
- depend on a UI
- mutate the trace contract
- classify every anomaly in agent execution
- replace general observability or debugging tools

## 13. Next Evolution

PCD can become more accurate when the trace surface becomes richer.

Expected improvements:

- richer spans with clearer step naming
- per-role emitters so actor boundaries are explicit
- usage and cost data present on more runs
- stronger span-to-span lineage across retries and handoffs

When those signals exist, PCD can move from conservative likelihood scoring toward more precise collapse classification.
