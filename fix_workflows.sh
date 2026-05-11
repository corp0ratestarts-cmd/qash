#!/usr/bin/env bash
# fix_workflows.sh
# Replaces deprecated actions-rs/toolchain@v1 with dtolnay/rust-toolchain@stable
# in all workflow files. Run from the root of the qash repo.

set -euo pipefail

# ---------------------------------------------------------------------------
# 0. Auth check
# ---------------------------------------------------------------------------
echo "==> Checking gh authentication..."
if ! gh auth status > /dev/null 2>&1; then
  echo "ERROR: Not logged in. Run: gh auth login --git-protocol https --web"
  exit 1
fi
gh auth setup-git
echo "  [OK] authenticated"

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# ---------------------------------------------------------------------------
# 1. Fix ci.yml
# ---------------------------------------------------------------------------
sed -i \
  's|uses: actions-rs/toolchain@v1\n.*toolchain: stable|uses: dtolnay/rust-toolchain@stable|g' \
  .github/workflows/ci.yml

# sed handles it line by line so do it in two passes
sed -i 's|uses: actions-rs/toolchain@v1|uses: dtolnay/rust-toolchain@stable|g' \
  .github/workflows/ci.yml

# Remove the now-orphaned "with:" + "toolchain: stable" block left behind
python3 - << 'PYEOF'
import re, pathlib

for wf in pathlib.Path(".github/workflows").glob("*.yml"):
    text = wf.read_text()
    # Remove:
    #       uses: dtolnay/rust-toolchain@stable   (already replaced)
    #       with:
    #         toolchain: stable
    cleaned = re.sub(
        r'(uses: dtolnay/rust-toolchain@stable)\s*\n\s*with:\s*\n\s*toolchain:\s*stable\s*\n',
        r'\1\n',
        text
    )
    if cleaned != text:
        wf.write_text(cleaned)
        print(f"  [OK] cleaned orphaned 'with:/toolchain:' block in {wf}")
    else:
        print(f"  [OK] no orphaned block found in {wf} (already clean)")
PYEOF

# ---------------------------------------------------------------------------
# 2. Fix platform-determinism.yml if it has the same issue
# ---------------------------------------------------------------------------
if grep -q "actions-rs/toolchain" .github/workflows/platform-determinism.yml 2>/dev/null; then
  sed -i 's|uses: actions-rs/toolchain@v1|uses: dtolnay/rust-toolchain@stable|g' \
    .github/workflows/platform-determinism.yml
  echo "  [OK] platform-determinism.yml updated"
else
  echo "  [OK] platform-determinism.yml already clean"
fi

# ---------------------------------------------------------------------------
# 3. Show the result so you can verify before committing
# ---------------------------------------------------------------------------
echo ""
echo "==> Result — ci.yml Setup Rust step:"
grep -A2 "Setup Rust" .github/workflows/ci.yml

echo ""
echo "==> Result — platform-determinism.yml Setup Rust step:"
grep -A2 "Setup Rust" .github/workflows/platform-determinism.yml 2>/dev/null || echo "  (step not found)"

# ---------------------------------------------------------------------------
# 4. Commit and push
# ---------------------------------------------------------------------------
echo ""
echo "==> Committing..."
git add .github/workflows/ci.yml .github/workflows/platform-determinism.yml
git commit -m "ci: replace deprecated actions-rs/toolchain with dtolnay/rust-toolchain

actions-rs/toolchain@v1 runs on Node.js 20 which is deprecated on GitHub
Actions from June 2026. dtolnay/rust-toolchain is the maintained replacement
and also eliminates the set-output deprecation warnings."

git push

echo ""
echo "==> Done. Watching CI..."
gh run watch
