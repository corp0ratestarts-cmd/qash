# Stabilization Governance

## Replay orchestration canonical entrypoint

The canonical replay orchestration entrypoint is `scripts/replay_test.sh`.

All replay orchestration for local development and CI should route through
`scripts/replay_test.sh` so replay behavior, flags, and reporting remain aligned.

## Replay wrapper lifecycle

New replay wrappers that duplicate `scripts/replay_test.sh` semantics are
prohibited by default. If a new wrapper is introduced, the change must also
include a deprecation and removal plan for any previous wrapper(s), including
migration notes for CI and developer workflows.
