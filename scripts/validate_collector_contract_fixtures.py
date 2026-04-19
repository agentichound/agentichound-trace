#!/usr/bin/env python
import json
import re
from pathlib import Path

import jsonschema


ROOT = Path(__file__).resolve().parents[1]
TRACE_SCHEMA_PATH = ROOT / "schemas" / "trace.schema.json"
VALID_FIXTURE = ROOT / "collector" / "fixtures" / "ingest-valid.json"
INVALID_FIXTURE = ROOT / "collector" / "fixtures" / "ingest-invalid.json"

MAX_PAYLOAD_BYTES = 524288
MAX_TRACES = 32
MAX_TOTAL_ENTITIES = 5000
MAX_SPANS_PER_TRACE = 1000
MAX_EVENTS_PER_TRACE = 2000
MAX_ERRORS_PER_TRACE = 200
MAX_USAGE_PER_TRACE = 1000
BATCH_ID_RE = re.compile(r"^bat_[A-Za-z0-9_-]+$")


def validate_envelope(doc: dict, payload_bytes: int) -> list[str]:
    errors: list[str] = []

    if payload_bytes > MAX_PAYLOAD_BYTES:
        errors.append("payload exceeds max_payload_bytes")

    if not isinstance(doc, dict):
        errors.append("envelope must be object")
        return errors

    required = {"batch_id", "sent_at", "traces"}
    missing = required.difference(doc.keys())
    for field in sorted(missing):
        errors.append(f"missing required field: {field}")

    allowed = {"batch_id", "sent_at", "traces"}
    extra = set(doc.keys()).difference(allowed)
    for field in sorted(extra):
        errors.append(f"unexpected field: {field}")

    batch_id = doc.get("batch_id")
    if batch_id is not None and not (isinstance(batch_id, str) and BATCH_ID_RE.match(batch_id)):
        errors.append("batch_id format invalid")

    traces = doc.get("traces")
    if traces is not None:
        if not isinstance(traces, list):
            errors.append("traces must be array")
        else:
            if len(traces) < 1:
                errors.append("traces must contain at least one trace")
            if len(traces) > MAX_TRACES:
                errors.append("traces exceeds max_traces_per_request")
            total_entities = 0
            for i, trace in enumerate(traces):
                if isinstance(trace, dict):
                    spans = trace.get("spans", [])
                    events = trace.get("events", [])
                    errs = trace.get("errors", [])
                    usage = trace.get("usage", [])
                    if isinstance(spans, list) and len(spans) > MAX_SPANS_PER_TRACE:
                        errors.append(f"trace[{i}] spans exceeds per-trace limit")
                    if isinstance(events, list) and len(events) > MAX_EVENTS_PER_TRACE:
                        errors.append(f"trace[{i}] events exceeds per-trace limit")
                    if isinstance(errs, list) and len(errs) > MAX_ERRORS_PER_TRACE:
                        errors.append(f"trace[{i}] errors exceeds per-trace limit")
                    if isinstance(usage, list) and len(usage) > MAX_USAGE_PER_TRACE:
                        errors.append(f"trace[{i}] usage exceeds per-trace limit")
                    if all(isinstance(x, list) for x in [spans, events, errs, usage]):
                        total_entities += 1 + len(spans) + len(events) + len(errs) + len(usage)
            if total_entities > MAX_TOTAL_ENTITIES:
                errors.append("total entities exceeds request limit")

    return errors


def validate_fixture(path: Path, trace_validator: jsonschema.Draft202012Validator) -> list[str]:
    payload = path.read_bytes()
    payload_bytes = len(payload)
    doc = json.loads(payload.decode("utf-8"))

    errors = validate_envelope(doc, payload_bytes)
    traces = doc.get("traces")
    if isinstance(traces, list):
        for i, trace in enumerate(traces):
            schema_errors = list(trace_validator.iter_errors(trace))
            for err in schema_errors:
                errors.append(f"trace[{i}] schema: {err.message}")

    return errors


def main() -> int:
    trace_schema = json.loads(TRACE_SCHEMA_PATH.read_text(encoding="utf-8"))
    trace_validator = jsonschema.Draft202012Validator(trace_schema)

    valid_errors = validate_fixture(VALID_FIXTURE, trace_validator)
    invalid_errors = validate_fixture(INVALID_FIXTURE, trace_validator)

    failed = False
    if valid_errors:
        failed = True
        print("FAIL: ingest-valid.json should pass but failed:")
        for err in valid_errors:
            print(f"- {err}")
    if not invalid_errors:
        failed = True
        print("FAIL: ingest-invalid.json should fail but passed")

    if failed:
        return 1

    print("PASS")
    print("ingest-valid.json: valid")
    print("ingest-invalid.json: invalid (as expected)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
