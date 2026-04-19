#!/usr/bin/env python
import json
from datetime import datetime, timezone
from pathlib import Path

import jsonschema


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "trace.schema.json"
EXAMPLES_DIR = ROOT / "schemas" / "examples"
INVALID_FILE = EXAMPLES_DIR / "invalid-missing-run-id.json"


def parse_ts(ts: str) -> datetime:
    if ts.endswith("Z"):
        ts = ts[:-1] + "+00:00"
    dt = datetime.fromisoformat(ts)
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt


def cross_checks(doc: dict, file_name: str) -> list[str]:
    errors: list[str] = []

    run = doc["run"]
    run_id = run["run_id"]

    run_start = parse_ts(run["started_at"])
    run_end = parse_ts(run["ended_at"])
    computed_run_ms = int((run_end - run_start).total_seconds() * 1000)
    if computed_run_ms != run["duration_ms"]:
        errors.append(f"{file_name}: run duration_ms mismatch")

    spans = doc["spans"]
    span_ids = {s["span_id"] for s in spans}

    for s in spans:
        sid = s["span_id"]
        if s["run_id"] != run_id:
            errors.append(f"{file_name}: span {sid} run_id mismatch")
        if s["parent_span_id"] is not None and s["parent_span_id"] not in span_ids:
            errors.append(f"{file_name}: span {sid} parent_span_id not found")
        if s["retry_of_span_id"] is not None and s["retry_of_span_id"] not in span_ids:
            errors.append(f"{file_name}: span {sid} retry_of_span_id not found")
        if s["kind"] == "retry" and s["retry_of_span_id"] is None:
            errors.append(f"{file_name}: retry span {sid} missing retry_of_span_id")
        if s["kind"] != "retry" and s["retry_of_span_id"] is not None:
            errors.append(f"{file_name}: non-retry span {sid} has retry_of_span_id")
        s_start = parse_ts(s["started_at"])
        s_end = parse_ts(s["ended_at"])
        computed_span_ms = int((s_end - s_start).total_seconds() * 1000)
        if computed_span_ms != s["duration_ms"]:
            errors.append(f"{file_name}: span {sid} duration_ms mismatch")
        if s_start < run_start or s_end > run_end:
            errors.append(f"{file_name}: span {sid} outside run time window")

    for e in doc["events"]:
        eid = e["event_id"]
        if e["run_id"] != run_id:
            errors.append(f"{file_name}: event {eid} run_id mismatch")
        if e["span_id"] not in span_ids:
            errors.append(f"{file_name}: event {eid} span_id not found")

    for err in doc["errors"]:
        erid = err["error_id"]
        if err["run_id"] != run_id:
            errors.append(f"{file_name}: error {erid} run_id mismatch")
        if err["span_id"] not in span_ids:
            errors.append(f"{file_name}: error {erid} span_id not found")

    for u in doc["usage"]:
        uid = u["usage_id"]
        if u["run_id"] != run_id:
            errors.append(f"{file_name}: usage {uid} run_id mismatch")
        if u["span_id"] not in span_ids:
            errors.append(f"{file_name}: usage {uid} span_id not found")
        if u["total_tokens"] != u["prompt_tokens"] + u["completion_tokens"]:
            errors.append(f"{file_name}: usage {uid} total_tokens mismatch")

    return errors


def main() -> int:
    with SCHEMA_PATH.open("r", encoding="utf-8") as f:
        schema = json.load(f)
    validator = jsonschema.Draft202012Validator(schema)

    valid_files = sorted(
        p for p in EXAMPLES_DIR.glob("*.json") if p.name != INVALID_FILE.name
    )
    expected_valid_count = 35
    if len(valid_files) != expected_valid_count:
        print(
            f"FAIL: expected {expected_valid_count} valid files, found {len(valid_files)}"
        )
        return 1

    all_errors: list[str] = []
    for path in valid_files:
        with path.open("r", encoding="utf-8") as f:
            doc = json.load(f)
        schema_errors = sorted(
            validator.iter_errors(doc), key=lambda e: list(e.path)
        )
        if schema_errors:
            for err in schema_errors:
                all_errors.append(f"{path.name}: {err.message}")
            continue
        all_errors.extend(cross_checks(doc, path.name))

    with INVALID_FILE.open("r", encoding="utf-8") as f:
        invalid_doc = json.load(f)
    invalid_errors = sorted(
        validator.iter_errors(invalid_doc), key=lambda e: list(e.path)
    )
    if not invalid_errors:
        all_errors.append("invalid-missing-run-id.json: expected validation failure")

    if all_errors:
        print("FAIL")
        for err in all_errors:
            print(f"- {err}")
        return 1

    print("PASS")
    print(f"valid files checked: {len(valid_files)}")
    print("invalid file correctly rejected: invalid-missing-run-id.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
