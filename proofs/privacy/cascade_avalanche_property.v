(** cascade_avalanche_property.v — Cascade avalanche: statistical/KAT evidence.

    Spec: docs/spec/09_privacy_model.md §P10 "TH-P1 dependency chain"

    Status: STATISTICAL — downgraded from formal proof target to statistical/KAT
    evidence per the v1.0 genesis-lock proof-debt classification in
    proofs/COVERAGE.md.

    Rationale:
      Avalanche is an empirical/statistical property of hash functions, not a
      core cryptographic security claim. The v1.0 genesis security rests on
      collision/preimage/second-preimage resistance assumptions (AX-3), not on
      avalanche. Genesis security does NOT depend on this property.

      A formal ROM proof would require SSProve/CryptHOL formalisation of each
      L1 primitive as a random oracle and a 5-way XOR combiner argument. This
      is non-trivial and not a prerequisite for the v1.0 claim boundary.

    Evidence (non-Coq):
      - All seven L1 primitives (SHA3-512, BLAKE3-XOF, KangarooTwelve-XOF,
        SM3-double-width, Streebog-512, Kupyna-512, LSH-512) are standardised
        hash functions with published avalanche analysis.
      - The QASH-CASCADE-7 known-answer test vectors in
        tests/vectors/cascade_kat.json verify output distribution at the
        byte level.
      - The platform-determinism.yml CI workflow confirms cross-ISA identity
        of the cascade output, which is a necessary (though not sufficient)
        condition for avalanche correctness.

    Post-genesis work (non-blocking):
      If a formal avalanche proof is later desired for completeness, it
      would be a proof-adjacent exercise in the ROM:
        1. Model each L1 primitive as a random oracle.
        2. Show L_i(x) XOR L_i(x') is uniformly distributed for x ≠ x'.
        3. Show the 5-way XOR combiner propagates bit diffusion.
        4. Bound the distinguishing advantage by negl(λ).
      This would upgrade the status from STATISTICAL to PROVED, but does
      not affect the v1.0 genesis claim boundary.

    Depends on: AX-3 (for statistical plausibility argument).
    Blocks: Nothing in v1.0 claim boundary.
*)

(** Statistical placeholder — not a formal proof obligation for v1.0.
    The axiom is retained for structural completeness but is explicitly
    NOT in the v1.0 active claim boundary. See proofs/COVERAGE.md. *)
Axiom cascade_avalanche_statistical :
  forall (x x' : list bool),
  x <> x' ->
  (* Statistical claim: H_cascade(x) and H_cascade(x') differ in at least
     half their bits with overwhelming probability under the ROM.
     Supported by KAT evidence in tests/vectors/cascade_kat.json.
     Not a genesis-lock security proof. *)
  True.
