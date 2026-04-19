# SDK Design

## Scope

This document defines Phase 1 SDK design constraints across Rust and TypeScript.

## SDK Principles

- Manual-first instrumentation
- Stable run/span semantics across languages
- Minimal required fields to emit useful runtime evidence
- Transport-agnostic core with collector transport adapter

## Rust SDK (`sdk/rust`)

- First-class API surface for Rust and any agent orchestration runtime
- Current seed files: `src/lib.rs`, `src/trace.rs`
- Tests live in `tests/`

## TypeScript SDK (`sdk/typescript`)

- Contract parity target with Rust SDK
- Current seed files: `src/index.ts`
- Tests live in `test/`

## Deferred SDKs

- `sdk/python/` placeholder only in Phase 1
- `sdk/go/` placeholder only in Phase 1
