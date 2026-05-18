#!/usr/bin/env bash
# fix_workflows.sh
set -euo pipefail

echo "==> Checking gh authentication..."
if ! gh auth status > /dev/null 2>&1; then
  echo "ERROR: Not logged in. Run: gh auth login --git-protocol https --web"
  exit 1
fi
gh auth setup-git
echo "  [OK] authenticated"

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

replace_in_file() {
  local file="$1"
  local from="$2"
  local to="$3"
  python3 - "$file" "$from" "$to" <<'PYEOF'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
from_s = sys.argv[2]
to_s = sys.argv[3]
text = path.read_text(encoding="utf-8")
new = text.replace(from_s, to_s)
if new != text:
    path.write_text(new, encoding="utf-8")
    print(f"  [OK] updated {path}")
else:
    print(f"  [OK] no replacement needed in {path}")
PYEOF
}

python_cleanup() {
python3 - <<'PYEOF'
import re, pathlib
for wf in pathlib.Path('.github/workflows').glob('*.yml'):
    text = wf.read_text(encoding='utf-8')
    cleaned = re.sub(
        r'(uses: dtolnay/rust-toolchain@stable)\s*\n\s*with:\s*\n\s*toolchain:\s*stable\s*\n',
        r'\1\n',
        text,
    )
    if cleaned != text:
        wf.write_text(cleaned, encoding='utf-8')
        print(f"  [OK] cleaned orphaned with/toolchain block in {wf}")
PYEOF
}

if [[ -f .github/workflows/ci.yml ]]; then
  replace_in_file .github/workflows/ci.yml "uses: actions-rs/toolchain@v1" "uses: dtolnay/rust-toolchain@stable"
else
  echo "  [WARN] .github/workflows/ci.yml missing; skipping"
fi

if [[ -f .github/workflows/platform-determinism.yml ]]; then
  replace_in_file .github/workflows/platform-determinism.yml "uses: actions-rs/toolchain@v1" "uses: dtolnay/rust-toolchain@stable"
else
  echo "  [WARN] .github/workflows/platform-determinism.yml missing; skipping"
fi

python_cleanup

echo ""
echo "==> Committing..."
mapfile -t wf_files < <(git diff --name-only -- .github/workflows/*.yml 2>/dev/null || true)
if [[ ${#wf_files[@]} -eq 0 ]]; then
  echo "No workflow changes detected; nothing to commit."
  exit 0
fi
git add "${wf_files[@]}"
git commit -m "ci: replace deprecated actions-rs/toolchain with dtolnay/rust-toolchain"
git push

echo ""
echo "==> Done. Watching CI..."
gh run watch
