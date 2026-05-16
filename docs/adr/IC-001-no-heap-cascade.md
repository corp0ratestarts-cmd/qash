# IC-001: No Heap Allocation in Cascade Verification

- **Status:** proposed
- **PDF anchor:** §3.1, pp. 5–6

## Verbatim PDF text

```text
let mut results = Vec::new();
```

## Constraint

When implemented in Domain A, cascade verification must not use heap allocation
or allocator-dependent behavior. PDF pseudocode using `Vec` is treated as
illustrative.

## Implementation rule

Use fixed-size arrays or statically bounded buffers, with explicit bounds tied
to genesis constants and checked at compile time or input validation time.

## Impact

This constraint does not change the PDF requirement to perform multi-primitive
verification. It only constrains how the requirement is implemented safely in
Domain A.
