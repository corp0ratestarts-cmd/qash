# /switch-repo

Switch the active Claude Code repository context to another GitHub repository visible to the current MCP session.

## Usage

```text
/switch-repo <owner/repo> [--branch <branch-or-ref>] [--path <subdir>]
```

## Behavior

1. Parse the target repository as `owner/repo`.
2. Check whether the repository is available in the active MCP/session scope.
3. If not visible, try repository discovery and report the missing scope clearly.
4. Confirm repository metadata and requested branch/ref.
5. Switch working context without modifying files.
6. Report the active repository, branch/ref, path, and known access level.

## Guardrails

- Do not infer a repository from a partial name when multiple matches exist.
- Do not create or modify files while switching context.
- Do not reset, checkout, pull, push, or mutate branches as part of the switch.
- Do not edit local permission files or credentials.
- If the target repository is not accessible, ask the user to add it to the MCP session.

## Output

```text
Active repository: <owner/repo>
Branch/ref: <branch-or-default>
Working path: <repo-root-or-subdir>
Access: <read/write/admin if known>
Next step: <one sentence>
```
