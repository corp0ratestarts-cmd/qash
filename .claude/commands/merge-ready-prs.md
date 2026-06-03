# /merge-ready-prs

Find open pull requests that are safe to merge, mark eligible drafts ready for review, and enable auto-merge using the repository's squash-merge policy.

## Usage

```text
/merge-ready-prs [--repo <owner/repo>] [--limit <n>] [--include-drafts] [--dry-run]
```

Examples:

```text
/merge-ready-prs
/merge-ready-prs --repo corp0ratestarts-cmd/qash --limit 20
/merge-ready-prs --include-drafts --dry-run
```

## Intent

Use this command to reduce manual PR queue management while preserving the repository's CI and review gates.

The command must:

1. Resolve the repository from `--repo` or the current active repository.
2. List open PRs, newest first, up to `--limit` or a safe default.
3. For each PR, inspect:
   - draft state;
   - mergeability;
   - head SHA;
   - required status checks and workflow conclusions;
   - review state, unresolved review threads, and requested changes where available;
   - branch protection or auto-merge support where available.
4. Treat advisory-only workflows as non-blocking only when repository documentation or branch protection makes that explicit.
5. Select only PRs where all required checks are green, no blocking review state exists, and the PR is mergeable.
6. If `--include-drafts` is present, mark eligible draft PRs ready for review before enabling auto-merge. Without it, report draft PRs but do not undraft them.
7. Enable auto-merge. Prefer SQUASH when the tool allows method selection; otherwise use the repository's configured auto-merge method and report that the connector inferred it.
8. Produce a summary table of merged/auto-merge-enabled/skipped PRs and exact skip reasons.

## Required safety rules

- Never merge a PR immediately unless the user explicitly says to merge now.
- Prefer enabling auto-merge over direct merge so branch protection remains authoritative.
- Do not undraft PRs unless `--include-drafts` is present or the user explicitly requested it.
- Do not bypass failing, pending, cancelled, skipped-required, or missing required checks.
- Do not treat unknown check state as green.
- Do not enable auto-merge on PRs with unresolved requested-changes reviews.
- Do not force-push, rebase, or update branches as part of this command.
- Do not close superseded PRs unless explicitly requested.

## Green-check interpretation

A PR is merge-ready only when:

```text
mergeable == true
state == open
required checks == success
blocking reviews == none
unresolved required review threads == none
```

When the GitHub connector exposes workflow runs rather than branch-protection status directly, inspect the head SHA workflow runs and summarize the result conservatively.

## Output format

Use this table:

| PR | Title | Head SHA | Action | Reason |
|----|-------|----------|--------|--------|
| #123 | Example | abc1234 | auto-merge enabled | all required checks green |
| #124 | Example | def5678 | skipped | pending CI |

End with:

```text
Processed: <n>
Auto-merge enabled: <n>
Marked ready: <n>
Skipped: <n>
```
