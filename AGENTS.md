# AGENTS.md

## Scope

These instructions apply to this repository. More specific `AGENTS.md` files in subdirectories override them.

Before changing a component, read the relevant package/config files and the applicable service or module documentation for that component.

Code and documentation changes should be scoped to the user’s request. Do not expand the architecture, introduce new systems, or make opportunistic refactors unless explicitly requested.

## Repository Map

- `src/` — Rust terminal application, state, and configuration.
- `src/aws/` — AWS CLI process control and service queries.
- `tests/` — UI checks and test data.
- `docs/` — Service and implementation guides.

## Commands

Run from the repository root:

- `cargo test` — Run the local test suite.
- `cargo test aws::` — Check AWS adapters and process control.
- `cargo fmt --all -- --check` — Check Rust formatting.
- `cargo clippy --all-targets -- -D warnings` — Check all targets for lint errors.
- `cargo build --release` — Build the release executable.
- `cargo run -- --demo` — Run with local sample data.
- No migration or code generation is required.

Do not report a command as passing unless it actually completed successfully. If a command is known to be unavailable, flaky, environment-dependent, or currently empty, state that clearly.

## Ponytail

Use the `ponytail` skill at its default `full` level for implementation, fixes, refactors, and reviews. Prefer the smallest correct change and do not add speculative abstractions or dependencies.

## Configuration

Define every configurable variable, constant, and default value inside the codebases' config file. Import these values where needed. Do not define configurable values elsewhere in the codebase.

## Code Spacing

Use consistent blank lines to make code easy to read. Put one blank line between functions and between groups of statements that do different tasks. Keep related statements together. Avoid large blocks with no blank lines, inconsistent spacing, and repeated blank lines.

## File Organization

When you write or change code, prefer focused files to one long file.

- Keep code concise and readable. Do not compress code or remove useful spacing to reduce the line count.
- Split files by purpose before they grow to hundreds of lines, such as 500 LOC. Use clear responsibilities to select file boundaries.


## Testing Philosophy

Write the minimum number of tests needed to protect meaningful behavior and prevent realistic regressions. Before adding tests, inspect existing coverage and avoid duplication. Prioritize observable behavior, critical paths, important edge cases, and regression tests for bugs. Do not test trivial logic, implementation details, framework behavior, or every possible input variation. Avoid excessive mocking, exhaustive test matrices, and tests written solely to increase coverage. Keep testing effort proportional to the risk and complexity of the change. Every new test should catch a distinct potential failure; stop adding tests once meaningful risks are covered. Small, low-risk changes do not automatically require new tests. Never remove valuable existing tests merely to reduce test count.

## Architecture Boundaries

Do not introduce new architecture without explicitly explaining why and asking first. Implement only the architecture requested.

## Documentation

Update the applicable README, service guide, or module documentation in the same change when modifying behavior, APIs, lifecycle, configuration, operations, or structure.

Keep durable architecture notes, workflows, and project-specific conventions in the appropriate documentation instead of growing this file unnecessarily.

## Commits

Commit messages must be specific, detailed, and production-level.

Use a precise Conventional Commit subject. Add a body when the change needs context, including the intent, affected behavior or scope, important implementation decisions, and verification.

Avoid generic messages such as `update docs`, `fix stuff`, or `misc changes`.
