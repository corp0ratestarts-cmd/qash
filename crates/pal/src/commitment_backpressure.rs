//! Commitment-only backpressure gate for Domain B ingress.
//!
//! The gate tracks bounded counters only. It does not store peer identity,
//! routes, topology, raw payloads, or transaction graph material.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureDecision {
    Admit,
    Throttle,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureError {
    ZeroLimit,
    CounterOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitmentBackpressure {
    admitted_in_window: u64,
    soft_limit: u64,
    hard_limit: u64,
}

impl CommitmentBackpressure {
    pub fn new(soft_limit: u64, hard_limit: u64) -> Result<Self, BackpressureError> {
        if soft_limit == 0 || hard_limit == 0 || soft_limit > hard_limit {
            return Err(BackpressureError::ZeroLimit);
        }
        Ok(Self {
            admitted_in_window: 0,
            soft_limit,
            hard_limit,
        })
    }

    pub fn admitted_in_window(&self) -> u64 {
        self.admitted_in_window
    }

    pub fn reset_window(&mut self) {
        self.admitted_in_window = 0;
    }

    pub fn observe_commitment(&mut self) -> Result<BackpressureDecision, BackpressureError> {
        if self.admitted_in_window >= self.hard_limit {
            return Ok(BackpressureDecision::Reject);
        }

        self.admitted_in_window = self
            .admitted_in_window
            .checked_add(1)
            .ok_or(BackpressureError::CounterOverflow)?;

        if self.admitted_in_window > self.soft_limit {
            Ok(BackpressureDecision::Throttle)
        } else {
            Ok(BackpressureDecision::Admit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_from_admit_to_throttle_to_reject() {
        let mut gate = CommitmentBackpressure::new(2, 4).unwrap();
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Admit
        );
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Admit
        );
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Throttle
        );
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Throttle
        );
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Reject
        );
    }

    #[test]
    fn reset_window_allows_new_admission() {
        let mut gate = CommitmentBackpressure::new(1, 1).unwrap();
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Admit
        );
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Reject
        );
        gate.reset_window();
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Admit
        );
    }
}
