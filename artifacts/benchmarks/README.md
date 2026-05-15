# Benchmark Evidence

This directory stores technical-effect measurements that may support patent and
engineering review.

## Candidate benchmark families

1. **Replay validation cost**: time and memory required to replay a fixed number
   of epochs.
2. **Validator stability evaluation cost**: cost as validator count approaches
   `MAX_VALIDATORS`.
3. **Divergence prevention**: number of intentionally contaminated inputs
   rejected before state mutation.
4. **Cross-ISA agreement**: count of target triples producing identical roots
   for the same replay vector.
5. **Halt-path determinism**: identical halt reasons for overflow, decode, or
   `δ_window` violations.

## Required fields

- Scenario and hypothesis.
- Commit hash.
- Machine and target triple.
- Rust toolchain.
- Command.
- Raw result table.
- Interpretation of technical effect.
- Link to replay artifact, if applicable.

## Minimal template

```text
# Benchmark: <scenario>

- Commit:
- Toolchain:
- Target triple:
- Command:
- Dataset or replay vector:
- Raw results:
- Technical effect observed:
- Limitations:
```
