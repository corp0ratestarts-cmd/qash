# /switch-repo

Switch the active Claude Code repository context to another GitHub repository that is visible to the current MCP session.

## Usage

```text
/switch-repo <owner/repo> [--branch <branch-or-ref>] [--path <subdir>]
```

Examples:

```text
/switch-repo corp0ratestarts-cmd/qash
/switch-repo corp0ratestarts-cmd/pure-qash --branch main
/switch-repo corp0ratestarts-cmd/qash --path crates/pal
```

## Intent

Use this command when work needs to move from the current repository to another repository without losing the session-level project context.

The command must:

1. Parse the target repository as `owner/repo`.
2. Check whether the repository is already available in the active MCP/session scope.
3. If the repository is not visible, attempt discovery with the available GitHub repository-list/search tool.
4. If the repository can be added to scope, add it before continuing.
5. Confirm access by fetching repository metadata and, when possible, the target branch.
6. Switch the working context to the requested repository, branch/ref, and optional subdirectory.
7. Report the active repository, branch/ref, and working path.

## MCP preference order

Prefer MCP/GitHub tools when available:

1. `list_repos` / repository search to discover available repositories.
2. `add_repo` / repository-scope addition if the target is not already in session scope.
3. Repository metadata fetch to confirm permissions and default branch.
4. Branch or ref lookup when `--branch` is supplied.

If a local checkout exists and MCP is unavailable, use local shell/git inspection only for read-only verification unless the user explicitly asks for local file edits.

## Guardrails

- Do not infer a repository from a partial name when multiple matches exist; show the candidates and stop.
- Do not create or modify files while switching context.
- Do not reset, checkout, pull, push, or mutate branches unless explicitly requested after the switch.
- Treat `~/.claude/settings.json` and permission files as out of scope unless the user directly asks to edit them and has granted local permission.
- If the target repository is not accessible to the session, explain the missing access and ask the user to add it to the Claude Code/MCP scope.

## Output format

Return a short status block:

```text
Active repository: <owner/repo>
Branch/ref: <branch-or-default>
Working path: <repo-root-or-subdir>
Access: <read/write/admin if known>
Next step: <one sentence>
```
