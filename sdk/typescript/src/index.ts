export type SpanKind =
  | "model"
  | "tool"
  | "orchestration"
  | "retry"
  | "handoff"
  | "approval";
export type SpanStatus = "ok" | "error" | "cancelled";

export interface Run {
  runId: string;
  startedAt: string;
}

export interface Span {
  spanId: string;
  runId: string;
  parentSpanId?: string;
  kind: SpanKind;
  name: string;
  status: SpanStatus;
  durationMs?: number;
}

export class Tracer {
  startRun(runId: string): Run {
    return {
      runId,
      startedAt: new Date().toISOString(),
    };
  }
}
