export type Status = "ok" | "error" | "cancelled";
export type SpanKind =
  | "model"
  | "tool"
  | "orchestration"
  | "retry"
  | "handoff"
  | "approval";
export type UsageKind = "model" | "tool";

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | { [key: string]: JsonValue }
  | JsonValue[];

export interface IngestRequest {
  batch_id: string;
  sent_at: string;
  traces: TraceDocument[];
}

export interface TraceDocument {
  schema_version: "v0";
  run: Run;
  spans: Span[];
  events: Event[];
  errors: TraceError[];
  usage: Usage[];
}

export interface Run {
  run_id: string;
  started_at: string;
  ended_at: string;
  duration_ms: number;
  status: Status;
}

export interface Span {
  span_id: string;
  run_id: string;
  parent_span_id: string | null;
  retry_of_span_id: string | null;
  kind: SpanKind;
  name: string;
  started_at: string;
  ended_at: string;
  duration_ms: number;
  status: Status;
}

export interface Event {
  event_id: string;
  run_id: string;
  span_id: string;
  kind: SpanKind;
  ts: string;
  payload: JsonValue;
}

export interface TraceError {
  error_id: string;
  run_id: string;
  span_id: string;
  ts: string;
  code: string;
  message: string;
  retryable: boolean;
}

export interface Usage {
  usage_id: string;
  run_id: string;
  span_id: string;
  kind: UsageKind;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  estimated_cost_usd: number;
  currency: string;
}

export interface EntityCounts {
  traces: number;
  runs: number;
  spans: number;
  events: number;
  errors: number;
  usage: number;
}

export interface IngestSuccessResponse {
  accepted: true;
  replayed: boolean;
  ingestion_id: string;
  batch_id: string;
  received: EntityCounts;
  inserted: EntityCounts;
  duplicates_ignored: EntityCounts;
}

export interface ErrorDetail {
  path: string;
  reason: string;
}

export interface ErrorResponse {
  accepted: false;
  code: string;
  message: string;
  details?: ErrorDetail[];
}

export interface RunSummary {
  run_id: string;
  status: Status;
  started_at: string;
  ended_at: string;
  duration_ms: number;
  span_count: number;
  event_count: number;
  error_count: number;
  usage_count: number;
  total_tokens: number;
  estimated_cost_usd: number;
  last_ingested_at: string;
}

export interface RunsResponse {
  runs: RunSummary[];
  next_cursor: string | null;
}

export interface RunDetailMeta {
  run_id: string;
  ingested_at: string;
}

export interface RunDetailResponse {
  trace: TraceDocument;
  meta: RunDetailMeta;
}
