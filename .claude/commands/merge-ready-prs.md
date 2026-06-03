# /merge-ready-prs

Find open pull requests that are safe to merge and perform only conservative merge actions.

## Usage

```text
/merge-ready-prs [--repo <owner/repo>] [--limit <n>] [--include-drafts] [--dry-run]
```

## Behavior

1. Resolve the target repository from `--repo` or current context.
2. List open PRs up to a safe limit.
3. Inspect draft state, mergeability, head SHA, workflow conclusions, review state, and unresolved review threads.
4. Select only PRs where required checks are green, no blocking review state exists, and the PR is mergeable.
5. If `--include-drafts` is present, mark eligible drafts ready before merge actions. Otherwise, report drafts and skip them.
6. Prefer auto-merge where available. If auto-merge is unavailable and the user explicitly asked to merge, use squash merge with expected head SHA.
7. Produce a table of actions and skip reasons.

## Safety rules

- Never bypass failing, pending, cancelled, skipped-required, or unknown checks.
- Never treat unknown check state as green.
- Never merge PRs with unresolved requested changes.
- Never undraft PRs unless explicitly requested.
- Never force-push, rebase, or update branches as part of this command.
- Prefer squash merge for direct merge actions.

## Output

| PR | Title | Head SHA | Action | Reason |
|----|-------|----------|--------|--------|
| #123 | Example | abc1234 | merged | all required checks green |
| #124 | Example | def5678 | skipped | pending CI |

```text
Processed: <n>
Merged or auto-merge enabled: <n>
Skipped: <n>
```
