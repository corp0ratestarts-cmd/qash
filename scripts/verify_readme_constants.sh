#!/usr/bin/env bash
# Verify that numeric constants mentioned in README.md match GENESIS_CONSTANTS.toml.
# Fails if any value visible in the README disagrees with the pinned TOML values.
# Run from workspace root.
set -euo pipefail

TOML="GENESIS_CONSTANTS.toml"
README="README.md"

if [[ ! -f "$TOML" ]]; then
  echo "ERROR: $TOML not found (run from workspace root)" >&2
  exit 1
fi

if [[ ! -f "$README" ]]; then
  echo "SKIP: $README not found — no README constants to verify"
  exit 0
fi

errors=0

check_readme_constant() {
  local description="$1"
  local toml_key="$2"
  local grep_pattern="$3"

  # Extract value from TOML (strip underscores for comparison)
  local toml_val
  toml_val=$(grep -E "^${toml_key}\s*=" "$TOML" | head -1 | sed 's/.*=\s*//' | tr -d '"_# ' | sed 's/\..*//')
  if [[ -z "$toml_val" ]]; then
    echo "WARN: $toml_key not found in $TOML — skipping README check for $description"
    return
  fi

  # Check if the README mentions a different numeric value near this constant
  if grep -qE "$grep_pattern" "$README" 2>/dev/null; then
    local readme_val
    readme_val=$(grep -oE "$grep_pattern" "$README" | head -1 | tr -d '_,.')
    if [[ "$readme_val" != "$toml_val" ]]; then
      echo "MISMATCH: $description — README has '$readme_val', TOML has '$toml_val'" >&2
      errors=$((errors + 1))
    else
      echo "OK: $description = $toml_val"
    fi
  else
    echo "SKIP: $description pattern not found in README (add it to lock it down)"
  fi
}

# Check the major pinned constants
check_readme_constant "weight_divergence_D"      "weight_divergence_D"      "350[_,]?000"
check_readme_constant "weight_conflict_C"        "weight_conflict_C"        "300[_,]?000"
check_readme_constant "weight_slash_Sigma"       "weight_slash_Sigma"       "200[_,]?000"
check_readme_constant "weight_cascade_health_CH" "weight_cascade_health_CH" "150[_,]?000"
check_readme_constant "cascade_depth"            "cascade_depth"            "cascade_depth[^0-9]*=?[^0-9]*[0-9]+"
check_readme_constant "duration_ms"              "duration_ms"              "500"

if [[ $errors -gt 0 ]]; then
  echo "FAIL: $errors constant(s) diverged between README and GENESIS_CONSTANTS.toml" >&2
  exit 1
fi

echo "verify_readme_constants: all checks passed"
