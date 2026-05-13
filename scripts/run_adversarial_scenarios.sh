#!/usr/bin/env bash
set -euo pipefail

# Scenario simulation hook runner.
# Intentionally targets explicitly named scenario tests when present.

cargo test --no-default-features adversarial_ -- --nocapture
