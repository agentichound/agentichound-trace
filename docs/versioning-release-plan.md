# Versioning and Release Plan

## Current Versioning Position

- Trace schema: `v0` (frozen)
- Collector contract: `v0` (frozen)
- Rust SDK crate: `0.x`
- TypeScript SDK package: `0.x`
- Collector crate: `0.x`

## Public Preview Tag Strategy

- Use `v0.1.0-alpha.1` for first public technical preview.
- Keep API-compatible implementation fixes within `v0.1.0-alpha.x`.
- Use `v0.1.0-beta.1` only after SDK ergonomics and docs are stable.

## Compatibility Policy (Preview)

- No breaking changes to frozen schema v0 and collector contract v0 during alpha unless a hard defect is found.
- Implementation changes are allowed if externally observable semantics remain unchanged.
- Any hard-defect break must be explicitly documented in release notes.

## Release Checklist (Preview)

- Contract tests passing
- Persistence/restart tests passing
- Fixture validation scripts passing
- Quickstart commands verified against runnable collector
