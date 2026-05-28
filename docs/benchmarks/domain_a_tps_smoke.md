# Domain A TPS Smoke Model

**Status:** Benchmark harness only. This is not a production throughput claim.

## Purpose

Domain A intentionally stays CPU-only so consensus state roots remain replayable,
portable, and independent of accelerator scheduling, GPU kernels, hardware entropy,
or device-specific timing. Hardware acceleration belongs in Domain B and must not
alter Domain A state-root semantics.

This benchmark answers the practical bottleneck question:

> Is CPU-only Domain A likely to become the TPS bottleneck, or can independent
> sharding provide enough aggregate capacity while Domain A remains deterministic?

## What it measures

The executable example `domain_a_tps_smoke` measures release-build
`advance_epoch` throughput for:

- idle epochs;
- max-divergence arithmetic epochs;
- full TX0 batches in ordered input order;
- full TX0 batches in reversed input order.

It prints:

- epochs per second;
- TX per second for TX-heavy scenarios;
- a simple independent-shard linear capacity model for shard counts such as
  `1,4,16,64`.

## What it does not measure

It does **not** measure:

- network throughput;
- production finality;
- cross-shard coordination cost;
- storage I/O;
- ZK proof generation or verification;
- GPU / accelerator performance;
- adversarial mempool behavior.

The shard model is a first-order capacity model only. Real global TPS requires
separate evidence for cross-shard receipt routing, EFB aggregation, networking,
operator hardware, and finality policy.

## Run

```bash
bash scripts/run_domain_a_tps_smoke.sh
```

Optional parameters:

```bash
ITERS=1000 WARMUP=100 SHARDS=1,4,16,64,256 \
  bash scripts/run_domain_a_tps_smoke.sh
```

The script writes a timestamped report under:

```text
artifacts/benchmarks/
```

## Interpreting results

Use the reversed full-batch result as the conservative Domain A TX-heavy
baseline, because it exercises the deterministic ordering path hardest.

If single-shard CPU-only TPS is already comfortably above the target per-shard
load, then hardware acceleration is not needed in Domain A.

If single-shard CPU-only TPS is low but the independent-shard model reaches the
target with a plausible shard count, the sharded architecture is likely the
right countermeasure.

If neither single-shard nor plausible-shard capacity reaches target TPS, then
optimize Domain A CPU paths before considering any architectural expansion.

## Evidence rules

Do not use this report as a public performance claim unless:

1. the benchmark is run on the exact reviewed commit;
2. machine specs are recorded;
3. results are archived under `artifacts/benchmarks/`;
4. cross-ISA state-root parity remains unchanged;
5. the claim is scoped to local CPU-only Domain A throughput, not production
   network TPS.
