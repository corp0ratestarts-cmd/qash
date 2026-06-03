# /repo-edit

Create or edit files in any repository that is in the current Claude Code/MCP session scope, using the safest available write path.

## Usage

```text
/repo-edit <owner/repo>:<path> [--branch <branch>] [--message <commit-message>]
/repo-edit local:<path>
```

Examples:

```text
/repo-edit corp0ratestarts-cmd/qash:docs/spec/19_profile_taxonomy.md
/repo-edit corp0ratestarts-cmd/pure-qash:Cargo.toml --branch chore/bootstrap
/repo-edit local:crates/pal/src/lib.rs
```

## Intent

Use this command for deliberate, reviewable repository edits across in-scope repositories. The command supports both remote GitHub writes through MCP and local checkout edits through Claude Code `Write`/`Edit` when local permissions allow.

## Target resolution

The command must:

1. Parse the target as one of:
   - `<owner/repo>:<path>` for a GitHub/MCP-backed target;
   - `local:<path>` for the current local checkout.
2. Resolve `--branch`; if omitted, use the current working branch for local targets or create a feature branch for remote targets unless the user explicitly asked to write to an existing branch.
3. Fetch existing file content before editing when the file exists.
4. Create missing parent directories when using local writes.
5. Use a minimal, focused patch.
6. Commit remotely through MCP or leave local edits unstaged unless the user asks for local git commits.
7. Report the changed path, branch, commit SHA if any, and whether a PR should be opened.

## Remote write preference order

For `<owner/repo>:<path>` targets:

1. Use `mcp__github__push_files` when available for multi-file or atomic batches.
2. Use GitHub contents create/update APIs for single-file writes when `push_files` is unavailable.
3. Never fall back to local writes for a different repository unless that checkout is explicitly present and requested.

## Local write preference order

For `local:<path>` targets:

1. Use Claude Code `Edit` for existing files.
2. Use Claude Code `Write` for new files.
3. Run formatting or tests only when directly relevant and safe.
4. Do not modify `~/.claude/settings.json`, command permissions, credential files, SSH keys, tokens, or other permission-policy files unless the user explicitly asks and local permission allows it.

## Branch and PR policy

- Prefer a new feature branch for remote writes.
- Do not write directly to `main` unless the user explicitly asks.
- Use atomic commits for related changes.
- Open a PR when the edit is complete unless the user asked only for a branch/commit.
- Include a clear test plan in the PR body.

## Safety rules

- Do not edit files outside the named repository/path target.
- Do not rewrite history or force-push unless the user explicitly requests it.
- Do not delete files unless the user explicitly asks.
- Do not change permission boundaries or model/tool access rules as part of a normal repo edit.
- Do not invent repository availability; if a repo is not in scope, ask the user to add it to the MCP session.
- For security-sensitive files, explain the risk and keep the diff minimal.

## Output format

```text
Repository: <owner/repo or local>
Branch: <branch>
Path(s): <changed paths>
Write path: <MCP push_files | GitHub contents API | local Edit/Write>
Commit: <sha or none>
PR: <url or not opened>
Tests: <run/not run and why>
```
