/// QASH FRI-STARK profile — type aliases and config constructor.
///
/// Profile ID: 0x0001_0001 (PLONKY3_FRI_POSEIDON_QASH, provisional v1.2).
///
/// # Parameters
///
/// | Parameter           | Value                                      |
/// |--------------------|--------------------------------------------|
/// | Field              | BabyBear (F_p, p = 2^31 − 2^27 + 1)      |
/// | Challenge field    | BabyBear^4 (degree-4 binomial extension)   |
/// | Inner circuit hash | Poseidon2/BabyBear, width 16, α=7          |
/// | Merkle arity       | 2-to-1, digest = 8 BabyBear elements       |
/// | PCS                | TwoAdicFriPcs (FRI over coset NTT)         |
/// | FRI log_blowup     | 1 (2× rate; ≈100-bit conjectured security) |
/// | FRI num_queries    | 100                                        |
/// | FRI PoW bits       | 16 (query phase)                           |
/// | Round constants    | Hardcoded (Grain LFSR; no trusted setup)   |
///
/// The round constants are exported from `p3-baby-bear` as
/// `default_babybear_poseidon2_16()` — deterministic, no RNG needed.
use p3_baby_bear::{default_babybear_poseidon2_16, BabyBear, Poseidon2BabyBear};
use p3_challenger::DuplexChallenger;
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2DitParallel;
use p3_field::extension::BinomialExtensionField;
use p3_field::Field;
use p3_fri::{create_test_fri_params, FriParameters, TwoAdicFriPcs};
use p3_merkle_tree::MerkleTreeMmcs;
use p3_symmetric::{PaddingFreeSponge, TruncatedPermutation};
use p3_uni_stark::StarkConfig;

// ── Field ────────────────────────────────────────────────────────────────────

pub type QashVal = BabyBear;

pub type QashChallenge = BinomialExtensionField<QashVal, 4>;

// ── Poseidon2 permutation (hardcoded round constants) ────────────────────────

pub type QashPerm = Poseidon2BabyBear<16>;

// ── Hash / compress for Merkle tree ─────────────────────────────────────────

/// PaddingFreeSponge<Perm, RATE=16, CAPACITY=8, OUTPUT=8>
pub type QashHash = PaddingFreeSponge<QashPerm, 16, 8, 8>;

/// TruncatedPermutation<Perm, N=2, CHUNK=8, WIDTH=16>
pub type QashCompress = TruncatedPermutation<QashPerm, 2, 8, 16>;

// ── MMCS (Merkle Mountain Commitment Scheme) ─────────────────────────────────

/// Merkle tree over base field: arity 2, digest width 8.
pub type QashValMmcs = MerkleTreeMmcs<
    <QashVal as Field>::Packing,
    <QashVal as Field>::Packing,
    QashHash,
    QashCompress,
    2,
    8,
>;

/// Extension-field MMCS wrapping the base-field MMCS.
pub type QashChallengeMmcs = ExtensionMmcs<QashVal, QashChallenge, QashValMmcs>;

// ── Challenger (Fiat-Shamir transcript) ─────────────────────────────────────

pub type QashChallenger = DuplexChallenger<QashVal, QashPerm, 16, 8>;

// ── DFT ─────────────────────────────────────────────────────────────────────

pub type QashDft = Radix2DitParallel<QashVal>;

// ── PCS ─────────────────────────────────────────────────────────────────────

pub type QashPcs = TwoAdicFriPcs<QashVal, QashDft, QashValMmcs, QashChallengeMmcs>;

// ── Full STARK config ────────────────────────────────────────────────────────

pub type QashFriConfig = StarkConfig<QashPcs, QashChallenge, QashChallenger>;

// ── FRI parameter constants ──────────────────────────────────────────────────

/// Production FRI parameters: 1-bit blowup, 100 queries, 16-bit query PoW.
/// Conjectured soundness: 100 bits (ethSTARK conjecture, log_blowup × num_queries).
pub const QASH_FRI_LOG_BLOWUP: usize = 1;
pub const QASH_FRI_NUM_QUERIES: usize = 100;
pub const QASH_FRI_COMMIT_POW_BITS: usize = 0;
pub const QASH_FRI_QUERY_POW_BITS: usize = 16;
pub const QASH_FRI_MAX_LOG_ARITY: usize = 1;
pub const QASH_FRI_LOG_FINAL_POLY_LEN: usize = 0;

// ── Config constructors ───────────────────────────────────────────────────────

/// Constructs the production QASH FRI-STARK config.
///
/// Uses hardcoded Poseidon2 round constants (Grain LFSR parameters, no trusted
/// setup) and production FRI parameters (100-bit conjectured security).
///
/// All nodes participating in the QASH protocol MUST use this configuration to
/// produce and verify proofs. Changing any parameter changes the proof format.
pub fn make_qash_production_config() -> QashFriConfig {
    let perm = default_babybear_poseidon2_16();
    build_config(perm, make_production_fri_params)
}

/// Constructs a fast QASH FRI-STARK config for unit tests.
///
/// Uses the same Poseidon2 round constants as production but minimal FRI
/// parameters (2 queries, 2-bit blowup) for fast test execution.
/// DO NOT use this config for proof generation outside of tests.
pub fn make_qash_test_config() -> QashFriConfig {
    let perm = default_babybear_poseidon2_16();
    build_config(perm, |mmcs| create_test_fri_params(mmcs, 2))
}

fn build_config<F>(perm: QashPerm, make_fri: F) -> QashFriConfig
where
    F: FnOnce(QashChallengeMmcs) -> FriParameters<QashChallengeMmcs>,
{
    let hash = QashHash::new(perm.clone());
    let compress = QashCompress::new(perm.clone());
    let val_mmcs = QashValMmcs::new(hash, compress, 0);
    let challenge_mmcs = QashChallengeMmcs::new(val_mmcs.clone());
    let dft = QashDft::default();
    let fri_params = make_fri(challenge_mmcs);
    let pcs = QashPcs::new(dft, val_mmcs, fri_params);
    let challenger = QashChallenger::new(perm);
    QashFriConfig::new(pcs, challenger)
}

fn make_production_fri_params(mmcs: QashChallengeMmcs) -> FriParameters<QashChallengeMmcs> {
    FriParameters {
        log_blowup: QASH_FRI_LOG_BLOWUP,
        log_final_poly_len: QASH_FRI_LOG_FINAL_POLY_LEN,
        max_log_arity: QASH_FRI_MAX_LOG_ARITY,
        num_queries: QASH_FRI_NUM_QUERIES,
        commit_proof_of_work_bits: QASH_FRI_COMMIT_POW_BITS,
        query_proof_of_work_bits: QASH_FRI_QUERY_POW_BITS,
        mmcs,
    }
}
