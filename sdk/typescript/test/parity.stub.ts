import type { IngestRequest, SpanKind, Status } from "../src/index.js";

const kinds: SpanKind[] = [
  "model",
  "tool",
  "orchestration",
  "retry",
  "handoff",
  "approval",
];

const statuses: Status[] = ["ok", "error", "cancelled"];

const envelope: IngestRequest = {
  batch_id: "bat_parity_stub",
  sent_at: "2026-04-19T19:00:00.000Z",
  traces: [],
};

if (kinds.length !== 6) {
  throw new Error("Span kind set drifted from Phase 1 contract.");
}

if (statuses.length !== 3) {
  throw new Error("Status set drifted from Phase 1 contract.");
}

if (!envelope.batch_id.startsWith("bat_")) {
  throw new Error("Envelope batch_id shape drifted from Phase 1 contract.");
}
