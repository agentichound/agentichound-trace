use crate::normalize::canonical_json;
use crate::types::{
    EntityCounts, IngestRequest, IngestSuccessResponse, RunDetailMeta, RunDetailResponse,
    RunSummary, RunsResponse, TraceDocument,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::path::Path;

#[derive(Debug)]
pub struct Storage {
    conn: Connection,
}

#[derive(Debug)]
pub enum StorageError {
    BatchIdConflict,
    EntityConflict(&'static str),
    SchemaVersionMismatch { expected: i64, found: i64 },
    Sqlite(String),
    Serialize(String),
    Deserialize(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::BatchIdConflict => write!(f, "batch_id reused with different payload"),
            StorageError::EntityConflict(kind) => write!(f, "{kind} entity conflict"),
            StorageError::SchemaVersionMismatch { expected, found } => {
                write!(
                    f,
                    "sqlite schema version mismatch: expected {expected}, found {found}"
                )
            }
            StorageError::Sqlite(msg) => write!(f, "sqlite error: {msg}"),
            StorageError::Serialize(msg) => write!(f, "serialization error: {msg}"),
            StorageError::Deserialize(msg) => write!(f, "deserialization error: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl Storage {
    const SQLITE_SCHEMA_VERSION: i64 = 1;

    pub fn in_memory() -> Result<Self, StorageError> {
        Self::open(":memory:")
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn =
            Connection::open(path).map_err(|e| StorageError::Sqlite(e.to_string()))?;
        let mut storage = Self { conn };
        storage.init()?;
        Ok(storage)
    }

    fn init(&mut self) -> Result<(), StorageError> {
        self.conn
            .execute_batch(
                r#"
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;

CREATE TABLE IF NOT EXISTS ingest_batches (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  batch_id TEXT NOT NULL UNIQUE,
  body_canonical TEXT NOT NULL,
  sent_at TEXT NOT NULL,
  ingestion_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runs (
  run_id TEXT PRIMARY KEY,
  trace_json TEXT NOT NULL,
  trace_canonical TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT NOT NULL,
  duration_ms INTEGER NOT NULL,
  span_count INTEGER NOT NULL,
  event_count INTEGER NOT NULL,
  error_count INTEGER NOT NULL,
  usage_count INTEGER NOT NULL,
  total_tokens INTEGER NOT NULL,
  estimated_cost_usd REAL NOT NULL,
  last_ingested_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS entity_runs (
  id TEXT PRIMARY KEY,
  canonical_payload TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS entity_spans (
  id TEXT PRIMARY KEY,
  canonical_payload TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS entity_events (
  id TEXT PRIMARY KEY,
  canonical_payload TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS entity_errors (
  id TEXT PRIMARY KEY,
  canonical_payload TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS entity_usage (
  id TEXT PRIMARY KEY,
  canonical_payload TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS schema_meta (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  schema_version INTEGER NOT NULL
);
"#,
            )
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT schema_version FROM schema_meta WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        match existing {
            None => {
                self.conn
                    .execute(
                        "INSERT INTO schema_meta (id, schema_version) VALUES (1, ?1)",
                        params![Self::SQLITE_SCHEMA_VERSION],
                    )
                    .map_err(|e| StorageError::Sqlite(e.to_string()))?;
            }
            Some(found) if found == Self::SQLITE_SCHEMA_VERSION => {}
            Some(found) => {
                return Err(StorageError::SchemaVersionMismatch {
                    expected: Self::SQLITE_SCHEMA_VERSION,
                    found,
                });
            }
        }
        Ok(())
    }

    pub fn ingest(
        &mut self,
        req: &IngestRequest,
        body_canonical: &str,
    ) -> Result<(u16, IngestSuccessResponse), StorageError> {
        if let Some((existing_body, ingestion_id)) = self
            .conn
            .query_row(
                "SELECT body_canonical, ingestion_id FROM ingest_batches WHERE batch_id = ?1",
                params![req.batch_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| StorageError::Sqlite(e.to_string()))?
        {
            if existing_body == body_canonical {
                return Ok((
                    200,
                    IngestSuccessResponse {
                        accepted: true,
                        replayed: true,
                        ingestion_id,
                        batch_id: req.batch_id.clone(),
                        received: receive_counts(req),
                        inserted: EntityCounts::default(),
                        duplicates_ignored: EntityCounts::default(),
                    },
                ));
            }
            return Err(StorageError::BatchIdConflict);
        }

        let tx = self
            .conn
            .transaction()
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        let mut inserted = EntityCounts::default();
        let mut duplicates_ignored = EntityCounts::default();
        let received = receive_counts(req);

        use std::collections::HashMap;
        let mut pending_runs: HashMap<String, String> = HashMap::new();
        let mut pending_spans: HashMap<String, String> = HashMap::new();
        let mut pending_events: HashMap<String, String> = HashMap::new();
        let mut pending_errors: HashMap<String, String> = HashMap::new();
        let mut pending_usage: HashMap<String, String> = HashMap::new();

        for trace in &req.traces {
            let run_payload =
                canonical_json(&trace.run).map_err(|e| StorageError::Serialize(e.to_string()))?;
            track_entity(
                &tx,
                "entity_runs",
                &trace.run.run_id,
                &run_payload,
                &mut pending_runs,
                &mut duplicates_ignored.runs,
                "run",
            )?;

            for span in &trace.spans {
                let payload =
                    canonical_json(span).map_err(|e| StorageError::Serialize(e.to_string()))?;
                track_entity(
                    &tx,
                    "entity_spans",
                    &span.span_id,
                    &payload,
                    &mut pending_spans,
                    &mut duplicates_ignored.spans,
                    "span",
                )?;
            }
            for event in &trace.events {
                let payload =
                    canonical_json(event).map_err(|e| StorageError::Serialize(e.to_string()))?;
                track_entity(
                    &tx,
                    "entity_events",
                    &event.event_id,
                    &payload,
                    &mut pending_events,
                    &mut duplicates_ignored.events,
                    "event",
                )?;
            }
            for err in &trace.errors {
                let payload =
                    canonical_json(err).map_err(|e| StorageError::Serialize(e.to_string()))?;
                track_entity(
                    &tx,
                    "entity_errors",
                    &err.error_id,
                    &payload,
                    &mut pending_errors,
                    &mut duplicates_ignored.errors,
                    "error",
                )?;
            }
            for usage in &trace.usage {
                let payload =
                    canonical_json(usage).map_err(|e| StorageError::Serialize(e.to_string()))?;
                track_entity(
                    &tx,
                    "entity_usage",
                    &usage.usage_id,
                    &payload,
                    &mut pending_usage,
                    &mut duplicates_ignored.usage,
                    "usage",
                )?;
            }
        }

        for (id, payload) in pending_runs {
            tx.execute(
                "INSERT INTO entity_runs (id, canonical_payload) VALUES (?1, ?2)",
                params![id, payload],
            )
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
            inserted.runs += 1;
        }
        for (id, payload) in pending_spans {
            tx.execute(
                "INSERT INTO entity_spans (id, canonical_payload) VALUES (?1, ?2)",
                params![id, payload],
            )
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
            inserted.spans += 1;
        }
        for (id, payload) in pending_events {
            tx.execute(
                "INSERT INTO entity_events (id, canonical_payload) VALUES (?1, ?2)",
                params![id, payload],
            )
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
            inserted.events += 1;
        }
        for (id, payload) in pending_errors {
            tx.execute(
                "INSERT INTO entity_errors (id, canonical_payload) VALUES (?1, ?2)",
                params![id, payload],
            )
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
            inserted.errors += 1;
        }
        for (id, payload) in pending_usage {
            tx.execute(
                "INSERT INTO entity_usage (id, canonical_payload) VALUES (?1, ?2)",
                params![id, payload],
            )
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
            inserted.usage += 1;
        }

        for trace in &req.traces {
            upsert_run(&tx, trace, &req.sent_at)?;
        }

        let seq: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM ingest_batches",
                [],
                |r| r.get(0),
            )
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        let ingestion_id = format!("ing_{:08}", seq);
        tx.execute(
            "INSERT INTO ingest_batches (batch_id, body_canonical, sent_at, ingestion_id) VALUES (?1, ?2, ?3, ?4)",
            params![req.batch_id, body_canonical, req.sent_at, ingestion_id],
        )
        .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        tx.commit()
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        Ok((
            201,
            IngestSuccessResponse {
                accepted: true,
                replayed: false,
                ingestion_id,
                batch_id: req.batch_id.clone(),
                received,
                inserted,
                duplicates_ignored,
            },
        ))
    }

    pub fn list_runs(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<RunsResponse, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT run_id, status, started_at, ended_at, duration_ms, span_count, event_count, error_count, usage_count, total_tokens, estimated_cost_usd, last_ingested_at
                 FROM runs
                 ORDER BY started_at DESC, run_id ASC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        let mut rows = stmt
            .query(params![limit as i64, offset as i64])
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().map_err(|e| StorageError::Sqlite(e.to_string()))? {
            out.push(RunSummary {
                run_id: r.get(0).map_err(|e| StorageError::Sqlite(e.to_string()))?,
                status: serde_json::from_str(&format!(
                    "\"{}\"",
                    r.get::<_, String>(1)
                        .map_err(|e| StorageError::Sqlite(e.to_string()))?
                ))
                .map_err(|e| StorageError::Deserialize(e.to_string()))?,
                started_at: r.get(2).map_err(|e| StorageError::Sqlite(e.to_string()))?,
                ended_at: r.get(3).map_err(|e| StorageError::Sqlite(e.to_string()))?,
                duration_ms: r.get(4).map_err(|e| StorageError::Sqlite(e.to_string()))?,
                span_count: r.get::<_, i64>(5).map_err(|e| StorageError::Sqlite(e.to_string()))?
                    as usize,
                event_count: r.get::<_, i64>(6).map_err(|e| StorageError::Sqlite(e.to_string()))?
                    as usize,
                error_count: r.get::<_, i64>(7).map_err(|e| StorageError::Sqlite(e.to_string()))?
                    as usize,
                usage_count: r.get::<_, i64>(8).map_err(|e| StorageError::Sqlite(e.to_string()))?
                    as usize,
                total_tokens: r.get(9).map_err(|e| StorageError::Sqlite(e.to_string()))?,
                estimated_cost_usd: r.get(10).map_err(|e| StorageError::Sqlite(e.to_string()))?,
                last_ingested_at: r.get(11).map_err(|e| StorageError::Sqlite(e.to_string()))?,
            });
        }

        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM runs", [], |r| r.get(0))
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        let next_cursor = if offset + out.len() < total as usize {
            Some(format!("c_{:08}", offset + out.len()))
        } else {
            None
        };
        Ok(RunsResponse {
            runs: out,
            next_cursor,
        })
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<RunDetailResponse>, StorageError> {
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT trace_json, last_ingested_at FROM runs WHERE run_id = ?1",
                params![run_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;

        let Some((trace_json, ingested_at)) = row else {
            return Ok(None);
        };
        let trace: TraceDocument =
            serde_json::from_str(&trace_json).map_err(|e| StorageError::Deserialize(e.to_string()))?;
        Ok(Some(RunDetailResponse {
            trace,
            meta: RunDetailMeta {
                run_id: run_id.to_string(),
                ingested_at,
            },
        }))
    }
}

fn receive_counts(req: &IngestRequest) -> EntityCounts {
    let mut counts = EntityCounts::default();
    counts.traces = req.traces.len();
    counts.runs = req.traces.len();
    for trace in &req.traces {
        counts.spans += trace.spans.len();
        counts.events += trace.events.len();
        counts.errors += trace.errors.len();
        counts.usage += trace.usage.len();
    }
    counts
}

fn track_entity(
    tx: &Transaction<'_>,
    table: &str,
    id: &str,
    canonical_payload: &str,
    pending: &mut std::collections::HashMap<String, String>,
    duplicate_counter: &mut usize,
    entity_kind: &'static str,
) -> Result<(), StorageError> {
    if let Some(existing_pending) = pending.get(id) {
        if existing_pending == canonical_payload {
            *duplicate_counter += 1;
            return Ok(());
        }
        return Err(StorageError::EntityConflict(entity_kind));
    }

    let query = format!("SELECT canonical_payload FROM {table} WHERE id = ?1");
    let existing: Option<String> = tx
        .query_row(&query, params![id], |r| r.get(0))
        .optional()
        .map_err(|e| StorageError::Sqlite(e.to_string()))?;
    if let Some(existing_payload) = existing {
        if existing_payload == canonical_payload {
            *duplicate_counter += 1;
            return Ok(());
        }
        return Err(StorageError::EntityConflict(entity_kind));
    }
    pending.insert(id.to_string(), canonical_payload.to_string());
    Ok(())
}

fn upsert_run(tx: &Transaction<'_>, trace: &TraceDocument, sent_at: &str) -> Result<(), StorageError> {
    let trace_json =
        serde_json::to_string(trace).map_err(|e| StorageError::Serialize(e.to_string()))?;
    let trace_canonical =
        canonical_json(trace).map_err(|e| StorageError::Serialize(e.to_string()))?;
    let total_tokens: i64 = trace.usage.iter().map(|u| u.total_tokens).sum();
    let estimated_cost_usd: f64 = trace.usage.iter().map(|u| u.estimated_cost_usd).sum();
    let status: String = serde_json::to_string(&trace.run.status)
        .map_err(|e| StorageError::Serialize(e.to_string()))?
        .trim_matches('"')
        .to_string();
    tx.execute(
        "INSERT INTO runs (run_id, trace_json, trace_canonical, status, started_at, ended_at, duration_ms, span_count, event_count, error_count, usage_count, total_tokens, estimated_cost_usd, last_ingested_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(run_id) DO UPDATE SET
           trace_json=excluded.trace_json,
           trace_canonical=excluded.trace_canonical,
           status=excluded.status,
           started_at=excluded.started_at,
           ended_at=excluded.ended_at,
           duration_ms=excluded.duration_ms,
           span_count=excluded.span_count,
           event_count=excluded.event_count,
           error_count=excluded.error_count,
           usage_count=excluded.usage_count,
           total_tokens=excluded.total_tokens,
           estimated_cost_usd=excluded.estimated_cost_usd,
           last_ingested_at=excluded.last_ingested_at",
        params![
            trace.run.run_id,
            trace_json,
            trace_canonical,
            status,
            trace.run.started_at,
            trace.run.ended_at,
            trace.run.duration_ms,
            trace.spans.len() as i64,
            trace.events.len() as i64,
            trace.errors.len() as i64,
            trace.usage.len() as i64,
            total_tokens,
            estimated_cost_usd,
            sent_at
        ],
    )
    .map_err(|e| StorageError::Sqlite(e.to_string()))?;
    Ok(())
}
