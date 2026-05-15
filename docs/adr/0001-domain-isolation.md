# ADR 0001: Domain Isolation as Protocol Law

## Status

Accepted.

## Context

QASH requires identical replay to produce identical state across authorized
execution environments. Operational services such as clocks, networking,
logging, hardware attestation, and acceleration are inherently nondeterministic
or platform-specific.

## Decision

QASH partitions execution into Domain A and Domain B. Domain A contains
consensus-visible transition semantics and is subject to replay invariance.
Domain B contains nondeterministic operational services. Domain B may provide
candidate inputs only through an admissibility gate, and any Domain B value that
changes Domain A semantics is nonconforming.

## Alternatives considered

- **Engineering convention only**: rejected because informal boundaries are hard
  to prove and easy to regress.
- **Single hosted runtime**: rejected because OS and hardware behavior would
  become part of the consensus proof surface.
- **Permit attestation-influenced semantics**: rejected because attestation
  results can be platform-specific and deployment-time dependent.

## Consequences

- Domain A remains smaller and more proof-eligible.
- Domain B can still optimize transport, logging, and hardware integration.
- Boundary tests and static checks become mandatory evidence for conformance.
