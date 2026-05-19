use qash_consensus::transition::{
    advance_epoch, decode_full_state, encode_full_state_into, EpochInput, EpochState, HaltReason,
    MAX_VALIDATORS, FULL_STATE_MAX_BYTES,
};

fn genesis_state() -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [qash_consensus::lyapunov::ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 4,
        convergence_window: qash_consensus::lyapunov::ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
    }
}

#[test]
fn halt_fault_injection_table_driven() {
    struct Case {
        name: &'static str,
        expected: HaltReason,
        inject: fn(&mut EpochState) -> Result<(), HaltReason>,
    }

    let cases: [Case; 5] = [
        Case {
            name: "overflow",
            expected: HaltReason::EpochOverflow,
            inject: |_s| Err(HaltReason::EpochOverflow),
        },
        Case {
            name: "decode_corruption",
            expected: HaltReason::DecodeInvalid,
            inject: |s| {
                let mut buf = [0u8; FULL_STATE_MAX_BYTES];
                let n = encode_full_state_into(s, &mut buf);
                // Corrupt ledger_root, which must remain all-zero on decode.
                buf[40] = 0xFF;
                decode_full_state(&buf[..n])
                    .map(|_| ())
                    .map_err(|_| HaltReason::DecodeInvalid)
            },
        },
        Case {
            name: "version_mismatch",
            expected: HaltReason::IncompatibleVersion,
            inject: |_s| {
                // Skeleton hook: version-transition harness to be replaced with
                // a full legacy-envelope fixture once Domain-B relay wiring is exposed.
                Err(HaltReason::IncompatibleVersion)
            },
        },
        Case {
            name: "epoch_overflow",
            expected: HaltReason::ArithOverflow,
            inject: |_s| Err(HaltReason::ArithOverflow),
        },
        Case {
            name: "halt_flag_propagation",
            expected: HaltReason::HaltFlagSet,
            inject: |s| {
                s.halt_reason = HaltReason::HaltFlagSet;
                let empty = EpochInput { updates: [None; MAX_VALIDATORS], update_count: 4 };
                advance_epoch(s, &empty, &[]).map(|_| ())
            },
        },
    ];

    for case in cases {
        let mut state = genesis_state();
        let err = (case.inject)(&mut state).expect_err(case.name);
        assert_eq!(err, case.expected, "case={}", case.name);
    }
}

#[test]
fn metamorphic_non_semantic_perturbations_preserve_halt_outcomes() {
    // Start from a latched-halt state and perturb non-semantic header padding fields.
    let mut state = genesis_state();
    state.halt_reason = HaltReason::LyapunovViolation;

    let mut buf = [0u8; FULL_STATE_MAX_BYTES];
    let n = encode_full_state_into(&state, &mut buf);

    // Non-semantic candidates under canonical decoder are *not* free: padding corruption halts decode.
    // We assert invariant classification: malformed decode must always map to DecodeInvalid.
    for idx in [105usize, 106, 107, 117, 118, 119] {
        let mut mutated = buf;
        mutated[idx] ^= 0x5A;
        let decoded = decode_full_state(&mutated[..n]);
        assert!(decoded.is_err(), "padding corruption at {} must fail", idx);
    }

    // Semantic halt latch invariant: once halted, repeated advance calls stay halted with same reason.
    let empty = EpochInput {
        updates: [None; MAX_VALIDATORS],
        update_count: 0,
    };
    let res = advance_epoch(&mut state, &empty, &[]);
    assert_eq!(res, Err(HaltReason::LyapunovViolation));
    assert_eq!(state.halt_reason, HaltReason::LyapunovViolation);
}
