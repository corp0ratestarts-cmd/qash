# /repo-edit

Create or edit files in any repository that is in the current Claude Code/MCP session scope, using the safest available write path.

## Usage

```text
/repo-edit <owner/repo>:<path> [--branch <branch>] [--message <commit-message>]
/repo-edit local:<path>
```

## Behavior

1. Parse the target as either `<owner/repo>:<path>` or `local:<path>`.
2. Resolve the target repository and branch.
3. Fetch existing content before editing when the file exists.
4. Use a minimal focused patch.
5. Prefer a new feature branch for remote writes unless the user explicitly asks for a direct write.
6. Report changed paths, branch, commit SHA if any, and PR status.

## Remote write preference

1. Use MCP/GitHub multi-file push support when available for atomic batches.
2. Use GitHub contents create/update APIs for single-file writes when multi-file push is unavailable.
3. Do not fall back to a local checkout for a different repository unless explicitly requested.

## Local write preference

1. Use local edit support for existing files.
2. Use local write support for new files.
3. Do not modify credentials, permission files, local settings, or tool access rules unless the user explicitly asks and local permission allows it.

## Branch and PR policy

- Prefer a new feature branch for remote writes.
- Do not write directly to `main` unless explicitly requested.
- Use atomic commits for related changes.
- Open a PR when the edit is complete unless the user asked only for a branch or commit.
- Include a clear test plan in the PR body.

## Safety rules

- Do not edit files outside the named repository/path target.
- Do not rewrite history or force-push unless explicitly requested.
- Do not delete files unless explicitly requested.
- Do not change permission boundaries as part of normal repo edits.
- If a repository is not in scope, ask the user to add it to the MCP session.

## Output

```text
Repository: <owner/repo or local>
Branch: <branch>
Path(s): <changed paths>
Write path: <MCP/GitHub/local>
Commit: <sha or none>
PR: <url or not opened>
Tests: <run/not run and why>
```
