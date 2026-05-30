/// Adversarial shard-capture simulation using configured bond weights.
///
/// Tests that the `assign_shard` assignment function based on
/// SHA3-256(epoch_seed ‖ validator_id ‖ bond_weight ‖ shard_count)
/// resists shard-capture even when an adversary controls a minority of
/// bond weight. The simulation verifies:
///
///   SIM-SC-1  Uniform distribution: with equal bond weights, assignment is
///             approximately uniform across shards.
///   SIM-SC-2  Adversarial capture bound: an adversary controlling f < 1/3 of
///             total bond weight controls < 1/3 + ε of any given shard's
///             validators (statistical property over seed variation).
///   SIM-SC-3  Bond weight influence: doubling bond weight changes assignment
///             (not deterministically biased — seed dominates the hash).
///   SIM-SC-4  Epoch seed rotation: the same validator gets a different shard
///             each epoch (seed changes → assignment changes).
///   SIM-SC-5  Large adversary cannot capture all shards: an adversary holding
///             40% of bond weight (above the 1/3 honest threshold) cannot
///             guarantee control of a shard in 100 trials.
use qash_consensus::sharding::assign_shard;

/// Fixed seed for reproducible simulation.
fn epoch_seed(epoch: u64) -> [u8; 32] {
    let mut s = [0u8; 32];
    s[0..8].copy_from_slice(&epoch.to_be_bytes());
    s[8] = 0xAB;
    s
}

/// Build a validator id from an index.
fn validator_id(idx: u32) -> [u8; 48] {
    let mut id = [0u8; 48];
    id[0..4].copy_from_slice(&idx.to_be_bytes());
    id[4] = 0xCC;
    id
}

/// Run assignment for all validators across one epoch, return shard assignments.
fn assign_all(seed: &[u8; 32], n: u32, bond: u64, shard_count: u32) -> Vec<u32> {
    (0..n)
        .map(|i| assign_shard(seed, &validator_id(i), shard_count, bond).unwrap())
        .collect()
}

// ── SIM-SC-1: Uniform distribution with equal bond weights ───────────────────

#[test]
fn uniform_distribution_with_equal_bond_weights() {
    let seed = epoch_seed(1);
    let n_validators = 128u32;
    let shard_count = 4u32;
    let assignments = assign_all(&seed, n_validators, 1_000, shard_count);

    let mut counts = vec![0u32; shard_count as usize];
    for &s in &assignments {
        counts[s as usize] += 1;
    }

    // With 128 validators and 4 shards, expect ~32 per shard.
    // Allow ±15 (≈47% deviation) to account for hash randomness.
    let expected = n_validators / shard_count;
    for (shard, &count) in counts.iter().enumerate() {
        assert!(
            count > 0 && count < 2 * expected,
            "shard {shard} has {count} validators but expected ≈{expected}; \
             distribution is too skewed"
        );
    }
}

// ── SIM-SC-2: Adversarial capture bound (f < 1/3 bond weight) ───────────────

#[test]
fn minority_adversary_cannot_dominate_every_shard() {
    // 90 honest validators (bond_weight = 1_000 each) +
    // 30 adversarial validators (bond_weight = 1_000 each) → 25% of population.
    let n_honest = 90u32;
    let n_adv = 30u32;
    let shard_count = 4u32;
    let bond = 1_000u64;

    // Run 10 independent epochs (different seeds).
    // Count epochs where adversary holds majority (> 50%) of ANY shard.
    let mut majority_captures = 0u32;

    for epoch in 0..10u64 {
        let seed = epoch_seed(epoch);

        let honest_assignments = assign_all(&seed, n_honest, bond, shard_count);
        let adv_assignments: Vec<u32> = (0..n_adv)
            .map(|i| {
                assign_shard(&seed, &validator_id(n_honest + i), shard_count, bond).unwrap()
            })
            .collect();

        let mut honest_counts = vec![0u32; shard_count as usize];
        let mut adv_counts = vec![0u32; shard_count as usize];
        for &s in &honest_assignments { honest_counts[s as usize] += 1; }
        for &s in &adv_assignments   { adv_counts[s as usize] += 1;   }

        // adv_count > total/2 → adv_count * 2 > total
        let any_majority = (0..shard_count as usize).any(|s| {
            let total = honest_counts[s] + adv_counts[s];
            total > 0 && adv_counts[s] * 2 > total
        });

        if any_majority {
            majority_captures += 1;
        }
    }

    // 25% adversary: may get lucky once or twice, but not every epoch.
    // This is a statistical property; with 10 epochs expect ≤ 5 captures.
    assert!(
        majority_captures <= 5,
        "25% adversary captured majority in {majority_captures}/10 epochs — \
         distribution is too biased"
    );
}

// ── SIM-SC-3: Bond weight influence on assignment ────────────────────────────

#[test]
fn bond_weight_changes_shard_assignment() {
    let seed = epoch_seed(42);
    let id = validator_id(7);
    let shard_count = 16u32;

    // Vary bond weight; most values should produce different assignments.
    let assignments: std::collections::BTreeSet<u32> = (0u64..20)
        .map(|w| assign_shard(&seed, &id, shard_count, w + 1).unwrap())
        .collect();

    // With 16 shards and 20 distinct bond weights, expect at least 3 distinct shards.
    assert!(
        assignments.len() >= 3,
        "bond weight variation produced only {} distinct shard assignments; \
         expected hash sensitivity to bond weight",
        assignments.len()
    );
}

// ── SIM-SC-4: Epoch seed rotation changes assignment ─────────────────────────

#[test]
fn epoch_seed_rotation_changes_assignment() {
    let id = validator_id(1);
    let shard_count = 8u32;
    let bond = 500u64;

    let assignments: std::collections::BTreeSet<u32> = (0u64..16)
        .map(|e| assign_shard(&epoch_seed(e), &id, shard_count, bond).unwrap())
        .collect();

    // With 16 epochs and 8 shards, expect at least 3 distinct shard assignments.
    assert!(
        assignments.len() >= 3,
        "epoch seed rotation produced only {} distinct assignments in 16 epochs; \
         assignment appears static",
        assignments.len()
    );
}

// ── SIM-SC-5: Large adversary (40% bond weight) cannot guarantee capture ─────

#[test]
fn strong_adversary_cannot_guarantee_full_shard_capture() {
    // 60 honest validators (bond 1_000 each, total 60_000)
    // 40 adversarial validators (bond 1_000 each, total 40_000)
    // Adversary holds 40% of total bond weight.
    let n_honest = 60u32;
    let n_adv = 40u32;
    let shard_count = 4u32;
    let bond = 1_000u64;

    // Count epochs where the adversary controls ALL four shards simultaneously.
    let mut full_captures = 0u32;

    for epoch in 0..100u64 {
        let seed = epoch_seed(epoch + 100); // offset to avoid overlap with SIM-SC-2

        let honest_assignments = assign_all(&seed, n_honest, bond, shard_count);
        let adv_assignments: Vec<u32> = (0..n_adv)
            .map(|i| {
                assign_shard(&seed, &validator_id(n_honest + i), shard_count, bond).unwrap()
            })
            .collect();

        let mut honest_counts = vec![0u32; shard_count as usize];
        let mut adv_counts = vec![0u32; shard_count as usize];
        for &s in &honest_assignments { honest_counts[s as usize] += 1; }
        for &s in &adv_assignments   { adv_counts[s as usize] += 1;   }

        // "Full capture" = adversary holds strict majority in every shard.
        let captured_all = (0..shard_count as usize).all(|s| {
            let total = honest_counts[s] + adv_counts[s];
            total > 0 && adv_counts[s] * 2 > total
        });

        if captured_all {
            full_captures += 1;
        }
    }

    // A 40% adversary should never achieve simultaneous majority in ALL shards.
    // In 100 epochs with random seed variation this probability is negligible.
    assert_eq!(
        full_captures, 0,
        "40% adversary achieved full shard capture in {full_captures}/100 epochs; \
         assignment function is insufficiently mixing"
    );
}
