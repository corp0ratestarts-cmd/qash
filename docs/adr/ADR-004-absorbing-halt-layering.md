# ADR-004: Absorbing Halt Layering Between Domain A and PAL

- **Status:** accepted
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

## Acceptance Evidence

- Domain A absorbing halt determinism and replayability are covered by
  `crates/pal/tests/halt_layering.rs`.
- PAL zeroization, scheduler-disable request, watchdog-reset request, and
  non-returning halt entrypoint ownership are covered by
  `pal_absorbing_halt_preparation_zeroizes_and_marks_domain_b_actions`.
- PAL halt preparation is covered as non-perturbing for Domain A roots by
  `pal_halt_preparation_cannot_perturb_domain_a_state_roots`.
