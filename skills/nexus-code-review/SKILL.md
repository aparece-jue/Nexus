---
name: nexus-code-review
description: Review Nexus code incrementally during development and review a completed module for architecture, maintainability and end-to-end behavior. Apply to Nexus code changes or explicit module acceptance reviews, not unrelated projects.
---

# Nexus code review

Use two review scopes: incremental checks for each change, and integrated review when a module or milestone is declared complete. Read the repository AGENTS.md and relevant accepted ADRs first; project rules and user instructions control conflicts. Do not reinterpret this skill as permission to install tools, publish, or rewrite unrelated code.

## Scope selection

- During development, review the current diff and its affected callers, contracts and tests. Use the task base commit or working diff, including intended untracked files; exclude personal data, generated output and unrelated user changes. Do not re-audit the entire repository on every edit.
- For module completion, identify the module, base/head commits, dependencies and real user scenario. Review all module changes plus necessary integration boundaries, rather than just the last commit. If scope is ambiguous, state a reasonable scope and inspect available history before asking.
- If a changed contract exposes a wider issue, extend review only along the demonstrated impact chain. Report unrelated pre-existing issues separately.

## Incremental review

Search existing interfaces and types before proposing new ones. Reuse compatible commands, validation, error handling, version gates and statistical rules. A new interface is justified by new semantics or a real boundary, not a different name for the same operation. Do not force distinct business operations through an incompatible old interface.

Check the actual path from the modified caller to storage and back where applicable:

- Correctness: states, inputs, units, errors, conflicts, cancellation and transaction boundaries remain consistent.
- Simplicity: no pure-forwarding layers, speculative generic frameworks, excessive nesting or duplicate facts. Extract a function for a coherent responsibility, not merely to satisfy a line limit.
- Readability: names express domain meaning; functions have understandable control flow; comments explain non-obvious reasons and constraints. Remove obsolete process commentary when touched.
- Boundaries: frontend presents backend facts; business rules live in Rust; modules do not directly mutate one another's tables; Agent uses the same controlled services.
- Compatibility: update affected Rust/TS contracts, migrations and callers together; no silent time-unit conversion, duplicate state source or unsafe reuse of stale responses.

Run existing checks appropriate to the change: formatting, type checking, lint, relevant regressions and build. Inspect configured tools before selecting commands; do not claim unconfigured lint rules are enforced. Avoid mechanical tests that mirror code. Pure documentation or low-impact display changes do not require full business regression. Fix issues introduced by the current task within scope, then rerun affected checks only.

## Module completion review

Review responsibility cohesion, dependency direction/cycles, duplicated APIs or rules, unnecessary abstractions, growing high-complexity areas and integration with adjacent modules. Metrics are signals, not reasons to split cohesive code arbitrarily. Do not invent hard thresholds or enable every strict lint without assessing noise.

Trace and, when implemented, execute a realistic scenario using real services, a temporary database and files. Substitute only external boundaries such as a controllable clock. Verify state, identity, versions, associations and aggregate results, not just success responses. Cover relevant interruption, duplicate/conflicting requests and failure recovery. For Nexus MVP follow docs/coding-plan-mvp.md and its simulation cases. Missing implementations remain explicit gaps.

For persistence modules include migration/reopen, backup integrity and recovery where affected. For UI verify actual access paths, errors and keyboard/focus behavior where relevant. For AI/file/network boundaries verify authorization and untrusted input handling. Service simulation, frontend mock tests and native desktop checks are separate evidence; none automatically proves the others.

## Evidence and outcome

For a routine change provide a short result: scope, material findings/fixes, checks actually run and any gap. Do not generate a report file for every small edit.

For a completed module record a concise report in the project's validation location with base/head commits, scenario, checks/results, actionable findings, remaining gaps and an acceptance conclusion. Each finding needs a concrete location, trigger, impact and recommended correction; distinguish blocker, follow-up and pre-existing issue. Do not call self-review independent review.

Use conclusions: ready within stated scope; changes required; or verification incomplete. Data loss, incorrect work-time accounting, broken source/version links and authorization bypass block affected module acceptance. Passing tests alone is insufficient if a required path is missing. Do not mark a module complete while required checks remain unexecuted, and do not expand this into an unrelated full security audit.

## Reference origins

These are supporting references, not runtime dependencies; read online only when clarification is needed:

- Google review guidance: https://github.com/google/eng-practices/tree/master/review
- Rust Clippy: https://github.com/rust-lang/rust-clippy
- ESLint rules: https://eslint.org/docs/latest/rules/
- Dependency rules: https://github.com/sverweij/dependency-cruiser
