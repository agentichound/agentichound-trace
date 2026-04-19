# Rust Emit E2E Demo

This example proves the first Rust emit path against a local collector without requiring a NYEX-specific runtime.

## Run

1. Start collector:
   - `cd collector`
   - `cargo run`
2. In a second terminal, emit and verify:
   - `cd sdk/rust`
   - `cargo run --example emit_e2e`

Optional:
- set `COLLECTOR_URL` if collector is not at `http://127.0.0.1:3000`
