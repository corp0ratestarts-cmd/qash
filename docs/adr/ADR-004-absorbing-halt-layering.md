# ADR-004: Absorbing Halt Layering Between Domain A and PAL

- **Status:** proposed
- **PDF anchor:** §2.3, pp. 3–4
- **Traceability rows:** P0-5

## Verbatim PDF text

```text
pub fn trigger_absorbing_halt(reason: HaltReason) -> ! {
    zeroize_critical_memory();
    #[cfg(feature = "itron")] unsafe { itron_disable_scheduler() };
    #[cfg(has_watchdog)] unsafe { trigger_wdt_reset() };
    loop { core::hint::spin_loop() }
}
```

## Context

The PDF presents halt as a single diverging function. The repository currently
has a deterministic consensus core and a platform abstraction layer, which means
halt behavior has both deterministic and platform-operational parts.

## Decision

Define the split explicitly:

- Domain A records or returns an absorbing halt state, freezes committed state,
  and rejects later inputs deterministically.
- Domain B / PAL performs zeroization, scheduler disablement, watchdog reset,
  and non-returning platform behavior.

## Consequences

The composition of Domain A and Domain B must satisfy the PDF's `-> !` contract
for deployed binaries, while keeping Domain A replayable and testable.
