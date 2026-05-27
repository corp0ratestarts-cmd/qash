#!/usr/bin/env bash
# audit_claim_boundary.sh — Phase 9 of the pre-genesis full-repo audit.
#
# Scans all .md/.toml/.txt files tracked by git for prohibited phrases that
# constitute claim overreach. Exits 1 on any unallowlisted match.
#
# Status: Blocking — exit 1 on any violation.
#
# Allowlist marker:
#   <!-- claim-boundary-allow: <reason> -->
#   Suppresses that line AND the immediately following line only.
#
# Excluded from scan:
#   docs/mvp/claims_register.md
#   docs/audit/**
#   docs/platforms/**
#   docs/release/**
#
# docs/funding/ and docs/compliance/ are NOT excluded — grant-facing and
# compliance-facing docs are exactly where overclaims are most dangerous.
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/claim_boundary.md"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Contextual overclaim patterns, case-insensitive grep -P regexes.
# Broad protocol words are not blocked alone because QASH legitimately uses
# capability-token, non-custodial, no-custody, and boundary-disclaimer language.
PROHIBITED_PHRASES=(
  'GDPR[[:space:]-]+compliant'
  'FIPS[[:space:]-]+validat'
  'FIPS[[:space:]-]+certif'
  'NSA[[:space:]-]+approv'
  'military[[:space:]-]+certif'
  'NATO[[:space:]-]+certif'
  'FedRAMP[[:space:]-]+authoris'
  'Common[[:space:]-]+Criteria[[:space:]-]+certif'
  'CMMC[[:space:]-]+compliant'
  'quantum[[:space:]-]+secure'
  'production[[:space:]-]+ready'
  'mainnet[[:space:]-]+ready'
  'financial[[:space:]-]+infrastructure'
  'payment[[:space:]-]+system'
  'settlement[[:space:]-]+layer'
  'investment[[:space:]-]+token'
  'utility[[:space:]-]+token'
  'security[[:space:]-]+token'
  'governance[[:space:]-]+token'
  'token[[:space:]-]+sale'
  'asset[[:space:]-]+custody'
  'funds[[:space:]-]+custody'
  'customer[[:space:]-]+custody'
  'custodial[[:space:]-]+service'
  'custody[[:space:]-]+of[[:space:]-]+assets'
)

PLATFORM_OVERCLAIMS=(
  'supports[[:space:]-]+all[[:space:]-]+platforms'
  'runs[[:space:]-]+on[[:space:]-]+all[[:space:]-]+platforms'
  'runs[[:space:]-]+on[[:space:]-]+every[[:space:]-]+platform'
  'runs[[:space:]-]+on[[:space:]-]+all[[:space:]-]+supported[[:space:]-]+platforms'
  'MUSA[[:space:]-]+support'
  'CUDA[[:space:]-]+support'
  'ROCm[[:space:]-]+support'
  'HSM[[:space:]-]+support'
  'TPM[[:space:]-]+support'
  'smartcard[[:space:]-]+support'
  'TEE[[:space:]-]+support'
  'full[[:space:]-]+RTOS[[:space:]-]+support'
)

mapfile -t SCAN_FILES < <(
  git ls-files '*.md' '*.toml' '*.txt' | grep -v \
    -e '^docs/mvp/claims_register\.md$' \
    -e '^docs/audit/' \
    -e '^docs/platforms/' \
    -e '^docs/release/'
)

echo "Scanning ${#SCAN_FILES[@]} files for prohibited claim patterns..."
echo "Scanning for platform overclaims outside docs/platforms/..."
perl - "$OUTPUT_FILE" "$COMMIT_SHA" "$TIMESTAMP" "${#SCAN_FILES[@]}" "${#PROHIBITED_PHRASES[@]}" "${#PLATFORM_OVERCLAIMS[@]}" "${SCAN_FILES[@]}" <<'PERL'
use strict;
use warnings;

my ($output_file, $commit_sha, $timestamp, $scan_count, $prohibited_count, $platform_count, @files) = @ARGV;

my @groups = (
  [
    'compliance/certification overclaim',
    qr/(?:GDPR[\s-]+compliant|FIPS[\s-]+validat|FIPS[\s-]+certif|NSA[\s-]+approv|military[\s-]+certif|NATO[\s-]+certif|FedRAMP[\s-]+authoris|Common[\s-]+Criteria[\s-]+certif|CMMC[\s-]+compliant|quantum[\s-]+secure|production[\s-]+ready|mainnet[\s-]+ready|financial[\s-]+infrastructure|payment[\s-]+system|settlement[\s-]+layer|investment[\s-]+token|utility[\s-]+token|security[\s-]+token|governance[\s-]+token|token[\s-]+sale|asset[\s-]+custody|funds[\s-]+custody|customer[\s-]+custody|custodial[\s-]+service|custody[\s-]+of[\s-]+assets)/i,
  ],
  [
    'platform overclaim',
    qr/(?:supports[\s-]+all[\s-]+platforms|runs[\s-]+on[\s-]+all[\s-]+platforms|runs[\s-]+on[\s-]+every[\s-]+platform|runs[\s-]+on[\s-]+all[\s-]+supported[\s-]+platforms|MUSA[\s-]+support|CUDA[\s-]+support|ROCm[\s-]+support|HSM[\s-]+support|TPM[\s-]+support|smartcard[\s-]+support|TEE[\s-]+support|full[\s-]+RTOS[\s-]+support)/i,
  ],
);

my $negative_context = qr/(^|[^[:alnum:]_])(?:not|no|non|never|without|must not|do not|cannot|should not|prohibit|prohibited|forbidden|blocked|avoid|no claim of|not a claim of|is not|are not)[^.\n]{0,160}/i;
my $example_context = qr/(^|[\s>#*-])(?:(?:the )?prohibited claims are|avoid\s*\(claim boundary violations\)|blocked:|blocked claims?|prohibited profile behavior|must not:|must never|do not use blocked terms)/i;
my $metadata_context = qr/^\*\*Branch:\*\*/i;
my $conditional_fips_context = qr/(?:FIPS validation applies .* requires lab validation|FIPS validation test report|should claim FIPS alignment .* until CMVP validation exists|running on FIPS-validated hardware RNG)/i;

my @violations;

for my $file (@files) {
  next unless -f $file;
  open my $fh, '<', $file or next;

  my $lineno = 0;
  my $skip_next = 0;
  my $example_lines = 0;

  while (my $line = <$fh>) {
    chomp $line;
    $lineno++;

    $example_lines = 0 if $line =~ /^#{1,3}\s+/;

    if ($line =~ $example_context) {
      $example_lines = 40;
      next;
    }

    if ($skip_next) {
      $skip_next = 0;
      next;
    }

    if (index($line, '<!-- claim-boundary-allow:') >= 0) {
      $skip_next = 1;
      next;
    }

    next if $line =~ $metadata_context;
    next if $line =~ $conditional_fips_context;

    for my $group (@groups) {
      my ($label, $pattern) = @$group;
      next unless $line =~ $pattern;

      if ($example_lines > 0) {
        $example_lines--;
        next;
      }
      next if $line =~ /$negative_context.*$pattern/;

      print "  VIOLATION: $file:$lineno: $line\n";
      push @violations, "$file:$lineno: $label";
    }

    $example_lines-- if $example_lines > 0;
  }
  close $fh;
}

open my $out, '>', $output_file or die "failed to write $output_file: $!";
my $fail = @violations ? 1 : 0;
print {$out} "# Claim Boundary Scan\n\n";
print {$out} "**Commit:** `$commit_sha`  \n";
print {$out} "**Timestamp:** $timestamp  \n";
print {$out} "**Status:** " . ($fail ? "❌ FAIL — violations found" : "✅ PASS — no violations") . "\n\n";
print {$out} "## Files scanned\n\n";
print {$out} "- **General scan:** $scan_count files (`.md`, `.toml`, `.txt` tracked by git, excluding exempt directories)\n";
print {$out} "- **Excluded:** `docs/mvp/claims_register.md`, `docs/audit/`, `docs/platforms/`, `docs/release/`\n";
print {$out} "- **NOT excluded:** `docs/funding/`, `docs/compliance/`\n\n";
print {$out} "## Pattern groups\n\n";
print {$out} "- Compliance/certification overclaim patterns: $prohibited_count\n";
print {$out} "- Platform overclaim patterns: $platform_count\n\n";
print {$out} "## Suppression policy\n\n";
print {$out} "Clearly negative uses and explicit blocked/prohibited/avoid example sections are not treated as live claims.\n";
print {$out} "The narrow allowlist marker remains available for one-off cases.\n\n";
if (@violations) {
  print {$out} "## Violations found\n\n";
  print {$out} "- `$_`\n" for @violations;
  print {$out} "\n";
}
print {$out} "## Allowlist marker\n\n";
print {$out} "A line containing `<!-- claim-boundary-allow: <reason> -->` suppresses\n";
print {$out} "that line and the **immediately following line only**. No broader suppression.\n\n";
print {$out} "## Verdict\n\n";
if ($fail) {
  print {$out} "**FAIL** — " . scalar(@violations) . " violation(s) found. Each must be removed,\n";
  print {$out} "corrected, or explicitly allowlisted with justification.\n";
} else {
  print {$out} "**PASS** — all scanned files are within the claim boundary.\n";
}
close $out;

print "\nClaim boundary scan complete.\n";
print "  Report: $output_file\n";
if ($fail) {
  print STDERR "  BLOCKING: " . scalar(@violations) . " violation(s) — see report for details.\n";
  exit 1;
}
print "  PASS: no violations found.\n";
PERL
