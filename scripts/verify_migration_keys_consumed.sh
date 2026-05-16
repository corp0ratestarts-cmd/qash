#!/usr/bin/env bash
# Verify that every key in [migration.compatibility] from GENESIS_CONSTANTS.toml
# is referenced (as a string literal or identifier) somewhere in the Rust source.
# Dead-letter config keys → immediate CI failure.
# Run from workspace root.
set -euo pipefail

TOML="GENESIS_CONSTANTS.toml"
RUST_SRC="src crates"

if [[ ! -f "$TOML" ]]; then
  echo "ERROR: $TOML not found (run from workspace root)" >&2
  exit 1
fi

# Extract keys from [migration.compatibility] section
keys=()
in_section=0
while IFS= read -r line; do
  trimmed="${line//[[:space:]]/}"
  if [[ "$trimmed" == "[migration.compatibility]" ]]; then
    in_section=1
    continue
  fi
  if [[ $in_section -eq 1 ]]; then
    # New section starts
    if [[ "$trimmed" =~ ^\[ ]]; then
      break
    fi
    # Skip comments and blank lines
    [[ "$trimmed" =~ ^# ]] && continue
    [[ -z "$trimmed" ]] && continue
    # Extract key name (before '=')
    key="${trimmed%%=*}"
    [[ -n "$key" ]] && keys+=("$key")
  fi
done < "$TOML"

if [[ ${#keys[@]} -eq 0 ]]; then
  echo "WARN: no keys found in [migration.compatibility] — section missing or empty?"
  exit 0
fi

echo "Found ${#keys[@]} migration.compatibility key(s): ${keys[*]}"

errors=0
for key in "${keys[@]}"; do
  # Search for the key as a string literal or bare identifier in Rust source
  if grep -rq --include="*.rs" -- "$key" $RUST_SRC 2>/dev/null; then
    echo "OK: '$key' referenced in Rust source"
  else
    echo "DEAD KEY: '$key' from [migration.compatibility] has no Rust reference" >&2
    errors=$((errors + 1))
  fi
done

if [[ $errors -gt 0 ]]; then
  echo "FAIL: $errors migration key(s) are dead-letter (not referenced in Rust)" >&2
  echo "      Either consume them in compat.rs or remove them from GENESIS_CONSTANTS.toml" >&2
  exit 1
fi

echo "verify_migration_keys_consumed: all keys consumed"
