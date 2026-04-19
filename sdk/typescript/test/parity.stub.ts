import type { SpanKind, SpanStatus } from "../src/index.js";

const kinds: SpanKind[] = [
  "model",
  "tool",
  "orchestration",
  "retry",
  "handoff",
  "approval",
];

const statuses: SpanStatus[] = ["ok", "error", "cancelled"];

if (kinds.length !== 6) {
  throw new Error("Span kind set drifted from Phase 1 contract.");
}

if (statuses.length !== 3) {
  throw new Error("Status set drifted from Phase 1 contract.");
}
