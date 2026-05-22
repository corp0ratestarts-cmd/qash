# QASH Benchmark Documentation

Benchmark documents in this directory record methodology and operating points
for claims that are not fully captured by unit tests or replay vectors.

Required future document:

- `zk_prover_sizing.md`: Domain B prover sizing evidence for Phase 6 sharding.
  It must record warm-up procedure, progressive load sweep, realistic
  transaction distributions, proof generation time, verification time, batch
  size, replica count, hardware profile, memory headroom, and the selected
  latency/throughput operating point.

Do not use this directory for raw benchmark dumps. Raw outputs belong under
`artifacts/benchmarks/`; docs here summarize the method and cite the archived
artifact path.
