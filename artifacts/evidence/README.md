# Evidence Archives

This directory stores review evidence captured for a specific commit or working
tree snapshot.

Evidence bundles are generated with:

```bash
bash scripts/capture_pre_genesis_evidence.sh
```

Each generated bundle should include:

- `manifest.txt`: commit identity, working-tree status, tool versions, and pass/fail summary.
- `cargo-metadata.json`: offline Cargo metadata used for supply-chain review.
- `logs/`: raw command output for replay, proof, PAL, Kani, hygiene, and supply-chain checks.

Use these bundles for pre-genesis review handoff only. They are not a genesis
lock decision and do not authorize performance, deployment, or production PAL
claims.
