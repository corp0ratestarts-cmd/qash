# Security Policy

## Supported versions

Only the `main` branch is actively maintained. Security fixes are not backported to prior commits.

## Reporting a vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report privately via GitHub's built-in security advisory mechanism:

1. Go to **Security → Advisories → Report a vulnerability** on this repository.
2. Describe the issue: affected component, reproduction steps, impact, and any suggested fix.

You will receive an acknowledgment within **48 hours** and a status update within **7 days**.

## Disclosure timeline

| Milestone | Target |
|-----------|--------|
| Acknowledgment | 48 hours |
| Triage & severity assignment | 7 days |
| Fix developed | 30 days (critical) / 90 days (all others) |
| Public disclosure | After fix is merged to `main` |

We follow coordinated disclosure. We will not take legal action against researchers acting in good faith.

## Scope

In scope for security reports:

- **Domain A (consensus core):** `crates/consensus/` — determinism breaks, arithmetic overflow bypasses, hash collision exploits, proof soundness violations
- **Domain B (PAL / hosted binary):** `crates/pal/`, `src/` — memory safety issues, unsafe code misuse
- **Cryptographic cascade:** any weakness in the 8-family hash cascade or GF(2¹²⁸) IT-MAC
- **Genesis lock bypass:** any mechanism to alter genesis constants without the required acknowledgment token
- **Cross-ISA non-determinism:** any input that produces different outputs on x86_64 vs aarch64 vs riscv64gc

Out of scope:

- Denial-of-service against the hosted binary (no production network exists yet)
- Issues in upstream dependencies — report those to the respective maintainers
- Social engineering attacks

## Severity classification

| Severity | Examples | Target fix time |
|----------|---------|-----------------|
| Critical | Consensus safety violation, secret key extraction, proof unsoundness | 30 days |
| High | Determinism break across ISAs, genesis lock bypass | 30 days |
| Medium | DoS in hosted binary, incorrect spec/code divergence | 90 days |
| Low | Information disclosure, spec inconsistency | 90 days |

## Contact

GitHub Security Advisories is the preferred channel. There is no PGP key or separate email address at this time.
