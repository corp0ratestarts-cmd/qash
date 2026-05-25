# Unlocked Domain A Demo Profile

**Status:** Design note for post-MVP hardening  
**Transaction:** `TX-MVP-ReceiptCommit`  
**Governing claims:** `docs/mvp/claims_register.md`  
**Related audit:** `docs/mvp/post_merge_audit.md`

## Purpose

The MVP incident receipt demonstrator needs a way to show admission, replay, and commitment-root evidence without claiming production consensus admission. This document defines the allowed shape of an **unlocked Domain A demo profile**.

The profile is a sandbox used for demonstrator evidence only. It must never be treated as a genesis-admitted transaction class, production settlement path, payment rail, identity system, or production critical-infrastructure deployment.

## Definitions

| Term | Meaning |
|------|---------|
| Locked Domain A | Genesis-admitted production consensus transition and constants. Changes here are protocol changes. |
| Unlocked Domain A demo profile | Non-genesis sandbox profile used to exercise deterministic admission and replay evidence. |
| Domain B | Private and operational material: receipt bodies, local vaults, disclosure bodies, I/O, tool artifacts, and demo CLI behavior. |

## Allowed profile behavior

The unlocked Domain A demo profile may:

1. accept `TxMvpReceiptCommitPublicExport` records as public transcript inputs;
2. validate fixed-size public-export encoding and version fields;
3. fold accepted public exports into a deterministic demo commitment root;
4. produce replay reports for funder, pilot, and audit evidence;
5. reject malformed, truncated, wrong-version, or reordered inputs;
6. run in CI and artifact scripts as a repeatable demonstrator lane.

## Prohibited profile behavior

The unlocked Domain A demo profile must not:

1. modify `GENESIS_CONSTANTS.toml`;
2. add `TX-MVP-ReceiptCommit` to a genesis-admitted transaction set;
3. change locked `advance_epoch` semantics;
4. accept private receipt bodies, vault contents, disclosure bundles, OS entropy, wall-clock time, or filesystem state as Domain A transition inputs;
5. create payment, settlement, custody, identity, credential, or hardware-attestation semantics;
6. be used as evidence of production ZK verification;
7. be described as production-ready, deployment-ready, certified, or genesis-locked.

## Public input boundary

The only admissible public input for the demo profile is the public commitment export form:

```text
TxMvpReceiptCommitPublicExport
```

This contains:

- version;
- epoch;
- transaction commitment;
- nonce commitment;
- payload commitment;
- disclosure-key commitment.

It must not contain:

- private receipt body;
- raw nonce;
- workspace salt;
- disclosure body;
- stable user identity;
- account or validator identity;
- filesystem path or local vault metadata.

## Replay root requirement

The profile should compute a deterministic root over public exports only. The root may be the same fold already used by the MVP CLI, or a later versioned profile-specific root, provided it is:

1. domain-separated;
2. deterministic across supported platforms;
3. independent of private body material;
4. independent of wall-clock, OS entropy, filesystem ordering, and nondeterministic map iteration;
5. documented with test vectors before implementation changes are merged.

## Implementation constraints

A future implementation PR must include:

1. a profile name or version constant that makes the sandbox status explicit;
2. a decode/validate path for public exports only;
3. deterministic ordering rules for public exports;
4. tests showing private body bytes cannot enter the profile;
5. tests showing imported public commitments support replay but not disclosure;
6. tests showing malformed public exports fail closed;
7. CI coverage through the MVP demo workflow;
8. claims-register review if any user-visible wording changes.

## Required tests before implementation merge

At minimum, add tests for:

| Test | Expected result |
|------|-----------------|
| valid public export sequence | accepted and produces stable root |
| wrong version | rejected |
| truncated export | rejected |
| extra bytes | rejected unless explicitly versioned as an envelope |
| private body injection attempt | rejected or impossible by type |
| raw nonce injection attempt | rejected or impossible by type |
| reordered sequence | root changes or deterministic ordering rule applies |
| imported-only workspace disclosure | rejected because private body is absent |

## Claims language

Allowed:

> The demonstrator uses an unlocked Domain A demo profile to exercise deterministic admission and replay evidence for public receipt commitments.

Blocked:

> `TX-MVP-ReceiptCommit` is admitted into locked Domain A consensus.

Blocked:

> The MVP is production-ready or genesis-admitted.

## Open design questions

1. Should the demo profile preserve input order, sort by `tx_commitment`, or sort by `(epoch, tx_commitment)`?
2. Should the public export format remain flat bytes, or move to a versioned envelope before pilot use?
3. Should replay reports include a profile hash that commits to this document and the profile implementation version?
4. Should the MVP demo profile live in `crates/consensus` behind a non-genesis feature flag, or remain in `crates/pal`/hosted code until a formal proof obligation exists?

## Recommendation

Start with a hosted, clearly non-genesis profile adapter that validates and replays public exports only. Promote any part of it toward consensus code only after the ordering rule, root definition, test vectors, and proof obligations are written down.
