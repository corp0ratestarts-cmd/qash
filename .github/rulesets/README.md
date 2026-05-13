# QASH GitHub rulesets

This directory stores the canonical ruleset payloads for the GitHub REST API.
They are intentionally committed so the `main` and release-tag protection policy
is reviewable alongside the code it protects.

Apply them with a repository administration token after replacing `OWNER` and
`REPO`:

```sh
curl -L \
  -X POST \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  https://api.github.com/repos/OWNER/REPO/rulesets \
  --data-binary @.github/rulesets/main-branch.json

curl -L \
  -X POST \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  https://api.github.com/repos/OWNER/REPO/rulesets \
  --data-binary @.github/rulesets/release-tags.json
```

Before enabling the `main` ruleset, replace `@your-username` in
`.github/CODEOWNERS` with the real maintainer account or team. The ruleset has
no bypass actors by design.
