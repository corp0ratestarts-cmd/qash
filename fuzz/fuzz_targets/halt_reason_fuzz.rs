use honggfuzz::fuzz;
use qash_consensus::transition::{
    advance_epoch, decode_full_state, encode_full_state_into, EpochInput, EpochState, HaltReason,
    FULL_STATE_MAX_BYTES,
};

fn main() {
    loop {
        fuzz!(|data: (u8, u8, [u8; 16])| {
            let (halt_code, mode, bytes) = data;
            let mut state = EpochState {
                epoch: 0,
                halt_reason: HaltReason::None,
                entropy_seed: [0u8; 32],
                validators: [qash_consensus::lyapunov::ValidatorMetrics::ZERO; qash_consensus::transition::MAX_VALIDATORS],
                validator_count: 4,
                convergence_window: qash_consensus::lyapunov::ConvergenceWindow::new(),
                nonces: [0u64; qash_consensus::transition::MAX_VALIDATORS],
                validator_ids: [[0u8; 48]; qash_consensus::transition::MAX_VALIDATORS],
                cascade_health: 0,
                state_root: [0u8; 32],
            };

            if let Ok(reason) = halt_from_u8(halt_code) {
                state.halt_reason = reason;
                let empty = EpochInput { updates: [None; qash_consensus::transition::MAX_VALIDATORS], update_count: 4 };
                let r = advance_epoch(&mut state, &empty, &[]);
                if reason != HaltReason::None {
                    assert_eq!(r, Err(reason));
                }
            }

            let mut buf = [0u8; FULL_STATE_MAX_BYTES];
            let n = encode_full_state_into(&state, &mut buf);

            if mode % 2 == 0 {
                let i = (bytes[0] as usize) % n;
                buf[i] ^= bytes[1];
            }

            let _ = decode_full_state(&buf[..n]);
        });
    }
}

fn halt_from_u8(v: u8) -> Result<HaltReason, ()> {
    match v {
        0x00 => Ok(HaltReason::None),
        0x01 => Ok(HaltReason::LyapunovViolation),
        0x02 => Ok(HaltReason::ArithOverflow),
        0x03 => Ok(HaltReason::EpochOverflow),
        0x04 => Ok(HaltReason::DecodeInvalid),
        0x05 => Ok(HaltReason::RoundtripFailure),
        0x06 => Ok(HaltReason::HaltFlagSet),
        0x07 => Ok(HaltReason::PhiSafetyViolation),
        0x08 => Ok(HaltReason::IncompatibleVersion),
        _ => Err(()),
    }
}
