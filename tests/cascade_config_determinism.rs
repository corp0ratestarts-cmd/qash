// Purity / determinism gate for select_cascade_config.
//
// select_cascade_config(epoch) must be a pure function: same epoch → same
// result always, and the result must exactly match the rotation schedule
// pinned in GENESIS_CONSTANTS.toml [crypto.cascade.rotation_schedule].

use qash::crypto::cascade_agility::select_cascade_config;

#[test]
fn cascade_config_is_deterministic() {
    let probe_epochs: &[u64] = &[0, 1, 9_999, 10_000, 19_999, 20_000, 29_999, 30_000, 39_999, 40_000, u64::MAX];
    for &epoch in probe_epochs {
        let a = select_cascade_config(epoch);
        let b = select_cascade_config(epoch);
        assert_eq!(a, b, "select_cascade_config({}) is not deterministic", epoch);
    }
}

#[test]
fn cascade_config_matches_genesis_toml_schedule() {
    // Values pinned from GENESIS_CONSTANTS.toml [crypto.cascade.rotation_schedule]
    let schedule: &[(u64, &str)] = &[
        (0,       "Dilithium5"),
        (9_999,   "Dilithium5"),
        (10_000,  "ML-DSA-87"),
        (19_999,  "ML-DSA-87"),
        (20_000,  "SLH-DSA-SHA3-256"),
        (29_999,  "SLH-DSA-SHA3-256"),
        (30_000,  "Falcon-512"),
        (39_999,  "Falcon-512"),
        (40_000,  "TERMINAL_HALT"),
    ];
    for &(epoch, expected) in schedule {
        assert_eq!(
            select_cascade_config(epoch), expected,
            "epoch {} → expected '{}' per GENESIS_CONSTANTS.toml", epoch, expected
        );
    }
}
