# Contributing

Keep changes aligned to the locked Phase 1 scope and canonical layout.

## Structure Rules

- Do not add new top-level folders beyond: `docs/`, `schemas/`, `sdk/`, `collector/`, `viewer/`, `cli/`, `examples/`, `scripts/`.
- Keep SDK language paths fixed: `sdk/rust`, `sdk/typescript`, `sdk/python`, `sdk/go`.
- Do not introduce parallel folder concepts that duplicate existing responsibilities.

## Scope Rules (Phase 1)

- Rust first, TypeScript second, Python and Go later.
- Trace first, Gateway later.
- No enterprise control-plane features, no platform expansion, no speculative abstractions.

## Contract Discipline

- Schema and collector contract changes come before implementation changes that depend on them.
- Avoid optional-field creep; add fields only when required by observed runtime evidence.
- Keep changes low-noise and implementation-first.

## Change Quality

- Prefer small, reviewable commits.
- Keep docs and examples updated with contract changes.
- Run local checks for touched components before opening a PR.
