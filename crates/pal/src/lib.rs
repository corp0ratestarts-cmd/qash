pub mod cap_token;
/// Platform Abstraction Layer (PAL) traits.
pub trait Time {
    fn epoch_counter() -> u64;
}
pub trait Net {
    fn send(_: &[u8]);
    fn recv(_: &mut [u8]) -> usize;
}
pub trait Attest {
    fn tpm_quote() -> [u8; 256];
}
pub trait Halt {
    fn absorbing_reset() -> !;
}

#[cfg(feature = "std")]
pub mod hosted {
    //! Hosted PAL runtime boundary.
    //!
    //! The hosted PAL is Domain B code: it can observe clocks, receive network
    //! frames, persist logs, collect attestation bytes, and request process
    //! halt/reset.  None of those observations are fed directly into Domain A.
    //! Domain A is entered only by replaying canonical [`CanonicalInput`]
    //! records through `qash_consensus::advance_epoch`.

    use super::*;
    use crate::cap_token::{validate_effect_token, CapTokenParams};
    use qash_consensus::{
        advance_epoch, validate_envelope_epoch, EpochInput, EpochState, FixedPoint, HaltReason,
        TransitionResult, ValidatorUpdate, MAX_VALIDATORS,
    };
    use std::collections::VecDeque;
    use std::fs::{File, OpenOptions};
    use std::io::{self, ErrorKind, Read, Write};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const LOG_MAGIC: &[u8; 8] = b"QPALOG1\0";
    const RECORD_MAGIC: &[u8; 8] = b"QPAIN1\0\0";
    const MAX_RAW_TX_BYTES: usize = 1 << 20;

    /// Domain-B-hosted runtime handle.
    ///
    /// `Host` owns only PAL-side resources.  Its time, transport,
    /// attestation, and halt/reset helpers do not mutate consensus state and
    /// are intentionally not consulted by [`apply_canonical_input`] or
    /// [`replay_from_genesis`].
    pub struct Host {
        log_path: PathBuf,
        inbound: VecDeque<Vec<u8>>,
        outbound: Vec<Vec<u8>>,
        attestation_quote: [u8; 256],
        reset_requested: bool,
    }

    /// A consensus-admissible input record after Domain B has normalized away
    /// transport, timing, and host metadata.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CanonicalInput {
        pub epoch: u64,
        pub updates: Vec<Option<CanonicalValidatorUpdate>>,
        pub raw_txs: Vec<Vec<u8>>,
    }

    /// Canonical fixed-point validator metric update.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CanonicalValidatorUpdate {
        pub divergence_raw: i64,
        pub conflict_raw: i64,
        pub slash_accum_raw: i64,
    }

    #[derive(Debug)]
    pub enum HostedError {
        Io(io::Error),
        InvalidLog(&'static str),
        InvalidInput(&'static str),
        ConsensusHalt(HaltReason),
    }

    impl From<io::Error> for HostedError {
        fn from(err: io::Error) -> Self {
            HostedError::Io(err)
        }
    }

    impl CanonicalInput {
        pub fn idle(epoch: u64, validator_count: u32) -> Result<Self, HostedError> {
            let count = usize::try_from(validator_count)
                .map_err(|_| HostedError::InvalidInput("validator_count overflows usize"))?;
            if count > MAX_VALIDATORS {
                return Err(HostedError::InvalidInput(
                    "validator_count exceeds MAX_VALIDATORS",
                ));
            }
            Ok(CanonicalInput {
                epoch,
                updates: vec![None; count],
                raw_txs: Vec::new(),
            })
        }

        fn to_epoch_input(&self, state: &EpochState) -> Result<EpochInput, HostedError> {
            if self.epoch != state.epoch {
                return Err(HostedError::InvalidInput(
                    "canonical input epoch does not match state",
                ));
            }
            let validator_count = usize::try_from(state.validator_count)
                .map_err(|_| HostedError::InvalidInput("state validator_count overflows usize"))?;
            if validator_count > MAX_VALIDATORS {
                return Err(HostedError::InvalidInput(
                    "state validator_count exceeds MAX_VALIDATORS",
                ));
            }
            if self.updates.len() != validator_count {
                return Err(HostedError::InvalidInput(
                    "canonical update count does not match state",
                ));
            }

            let mut updates = [None; MAX_VALIDATORS];
            for (idx, update) in self.updates.iter().enumerate() {
                updates[idx] = match update {
                    Some(u) => Some(ValidatorUpdate {
                        divergence_new: FixedPoint::from_raw(i128::from(u.divergence_raw)),
                        conflict_new: FixedPoint::from_raw(i128::from(u.conflict_raw)),
                        slash_accum_new: FixedPoint::from_raw(i128::from(u.slash_accum_raw)),
                    }),
                    None => None,
                };
            }

            Ok(EpochInput {
                updates,
                update_count: state.validator_count,
            })
        }

        fn validate_with_cap_tokens(&self, state: &EpochState) -> Result<(), HostedError> {
            validate_envelope_epoch(self.epoch, 0, state.epoch, 1)
                .map_err(|_| HostedError::InvalidInput("envelope epoch outside admission window"))?;

            let params = CapTokenParams {
                max_validators: state.validator_count,
                ..CapTokenParams::default()
            };

            // Validate update-bearing validator effects.
            for (validator_id, update) in self.updates.iter().enumerate() {
                if update.is_some() {
                    let _ = validate_effect_token(
                        &params,
                        self.epoch.saturating_add(1),
                        validator_id as u32,
                        state.cascade_health,
                        &[],
                    ).map_err(|_| HostedError::InvalidInput("capability token validation failed for validator update"))?;
                }
            }

            // Validate raw tx effects crossing Domain B boundary.
            for tx in &self.raw_txs {
                let _ = validate_effect_token(
                    &params,
                    self.epoch.saturating_add(1),
                    0,
                    state.cascade_health,
                    tx.as_slice(),
                ).map_err(|_| HostedError::InvalidInput("capability token validation failed for raw tx"))?;
            }

            Ok(())
        }

    }

    impl Host {
        pub fn new(log_path: impl Into<PathBuf>) -> Result<Self, HostedError> {
            let log_path = log_path.into();
            ensure_log_header(&log_path)?;
            Ok(Host {
                log_path,
                inbound: VecDeque::new(),
                outbound: Vec::new(),
                attestation_quote: [0u8; 256],
                reset_requested: false,
            })
        }

        pub fn log_path(&self) -> &Path {
            &self.log_path
        }

        /// Apply and persist one canonical Domain-A input.
        ///
        /// The record is appended only after `advance_epoch` accepts it.  This
        /// means a restarted host replays exactly the accepted transition log;
        /// rejected host/network/time observations are not durable Domain-A
        /// inputs.
        pub fn apply_canonical_input(
            &mut self,
            state: &mut EpochState,
            input: &CanonicalInput,
        ) -> Result<TransitionResult, HostedError> {
            input.validate_with_cap_tokens(state)?;
            let epoch_input = input.to_epoch_input(state)?;
            let raw_refs: Vec<&[u8]> = input.raw_txs.iter().map(Vec::as_slice).collect();

            // Run the deterministic transition on a scratch copy first.  The
            // accepted record is made durable before publishing the resulting
            // Domain A state to the caller, so an append failure cannot leave
            // in-memory state ahead of the recoverable log.
            let mut next_state = *state;
            let result = advance_epoch(&mut next_state, &epoch_input, &raw_refs)
                .map_err(HostedError::ConsensusHalt)?;
            append_record(&self.log_path, input)?;
            *state = next_state;
            Ok(result)
        }

        /// Replay the persisted canonical log from a supplied genesis state.
        pub fn replay_from_genesis(&self, genesis: EpochState) -> Result<EpochState, HostedError> {
            let mut state = genesis;
            for input in read_records(&self.log_path)? {
                input.validate_with_cap_tokens(&state)?;
                let epoch_input = input.to_epoch_input(&state)?;
                let raw_refs: Vec<&[u8]> = input.raw_txs.iter().map(Vec::as_slice).collect();
                advance_epoch(&mut state, &epoch_input, &raw_refs)
                    .map_err(HostedError::ConsensusHalt)?;
            }
            Ok(state)
        }

        /// Domain B transport ingress. Bytes are queued for host processing;
        /// they are never interpreted as consensus input by this method.
        pub fn enqueue_network_frame(&mut self, frame: Vec<u8>) {
            self.inbound.push_back(frame);
        }

        /// Domain B transport receive. Returns a queued frame without touching
        /// Domain A state.
        pub fn recv_network_frame(&mut self) -> Option<Vec<u8>> {
            self.inbound.pop_front()
        }

        /// Domain B transport send. Captures bytes for the hosted adapter only.
        pub fn send_network_frame(&mut self, frame: &[u8]) {
            self.outbound.push(frame.to_vec());
        }

        pub fn sent_frames(&self) -> &[Vec<u8>] {
            &self.outbound
        }

        pub fn set_attestation_quote(&mut self, quote: [u8; 256]) {
            self.attestation_quote = quote;
        }

        pub fn attestation_quote(&self) -> [u8; 256] {
            self.attestation_quote
        }

        pub fn request_reset(&mut self) {
            self.reset_requested = true;
        }

        pub fn reset_requested(&self) -> bool {
            self.reset_requested
        }
    }

    impl Time for Host {
        fn epoch_counter() -> u64 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        }
    }

    impl Net for Host {
        fn send(_: &[u8]) {}
        fn recv(_: &mut [u8]) -> usize {
            0
        }
    }

    impl Attest for Host {
        fn tpm_quote() -> [u8; 256] {
            [0u8; 256]
        }
    }

    impl Halt for Host {
        fn absorbing_reset() -> ! {
            std::process::abort()
        }
    }

    fn ensure_log_header(path: &Path) -> Result<(), HostedError> {
        match File::open(path) {
            Ok(mut file) => {
                let mut magic = [0u8; 8];
                file.read_exact(&mut magic)?;
                if &magic != LOG_MAGIC {
                    return Err(HostedError::InvalidLog("invalid hosted PAL log magic"));
                }
                Ok(())
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
                file.write_all(LOG_MAGIC)?;
                file.sync_all()?;
                Ok(())
            }
            Err(err) => Err(HostedError::Io(err)),
        }
    }

    fn append_record(path: &Path, input: &CanonicalInput) -> Result<(), HostedError> {
        let payload = encode_input(input)?;
        let mut file = OpenOptions::new().append(true).open(path)?;
        file.write_all(RECORD_MAGIC)?;
        file.write_all(&(payload.len() as u32).to_le_bytes())?;
        file.write_all(&payload)?;
        file.sync_all()?;
        Ok(())
    }

    fn read_records(path: &Path) -> Result<Vec<CanonicalInput>, HostedError> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)?;
        if &magic != LOG_MAGIC {
            return Err(HostedError::InvalidLog("invalid hosted PAL log magic"));
        }

        let mut records = Vec::new();
        loop {
            let mut record_magic = [0u8; 8];
            match file.read_exact(&mut record_magic) {
                Ok(()) => {}
                Err(err) if err.kind() == ErrorKind::UnexpectedEof => break,
                Err(err) => return Err(HostedError::Io(err)),
            }
            if &record_magic != RECORD_MAGIC {
                return Err(HostedError::InvalidLog(
                    "invalid canonical input record magic",
                ));
            }
            let mut len_bytes = [0u8; 4];
            file.read_exact(&mut len_bytes)?;
            let len = u32::from_le_bytes(len_bytes) as usize;
            let mut payload = vec![0u8; len];
            file.read_exact(&mut payload)?;
            records.push(decode_input(&payload)?);
        }
        Ok(records)
    }

    fn encode_input(input: &CanonicalInput) -> Result<Vec<u8>, HostedError> {
        if input.updates.len() > MAX_VALIDATORS {
            return Err(HostedError::InvalidInput("too many canonical updates"));
        }
        let mut out = Vec::new();
        out.extend_from_slice(&input.epoch.to_le_bytes());
        out.extend_from_slice(&(input.updates.len() as u32).to_le_bytes());
        out.extend_from_slice(&(input.raw_txs.len() as u32).to_le_bytes());
        for update in &input.updates {
            match update {
                Some(u) => {
                    out.push(1);
                    out.extend_from_slice(&[0u8; 3]);
                    out.extend_from_slice(&u.divergence_raw.to_le_bytes());
                    out.extend_from_slice(&u.conflict_raw.to_le_bytes());
                    out.extend_from_slice(&u.slash_accum_raw.to_le_bytes());
                }
                None => {
                    out.push(0);
                    out.extend_from_slice(&[0u8; 27]);
                }
            }
        }
        for tx in &input.raw_txs {
            if tx.len() > MAX_RAW_TX_BYTES {
                return Err(HostedError::InvalidInput(
                    "raw transaction exceeds hosted PAL limit",
                ));
            }
            out.extend_from_slice(&(tx.len() as u32).to_le_bytes());
            out.extend_from_slice(tx);
        }
        Ok(out)
    }

    fn decode_input(bytes: &[u8]) -> Result<CanonicalInput, HostedError> {
        if bytes.len() < 16 {
            return Err(HostedError::InvalidLog(
                "canonical input record is too short",
            ));
        }
        let mut pos = 0;
        let epoch = read_u64(bytes, &mut pos)?;
        let update_count = read_u32(bytes, &mut pos)? as usize;
        let tx_count = read_u32(bytes, &mut pos)? as usize;
        if update_count > MAX_VALIDATORS {
            return Err(HostedError::InvalidLog(
                "canonical update count exceeds MAX_VALIDATORS",
            ));
        }
        let mut updates = Vec::with_capacity(update_count);
        for _ in 0..update_count {
            if pos + 28 > bytes.len() {
                return Err(HostedError::InvalidLog("truncated canonical update"));
            }
            let present = bytes[pos];
            if bytes[pos + 1] != 0 || bytes[pos + 2] != 0 || bytes[pos + 3] != 0 {
                return Err(HostedError::InvalidLog("non-canonical update padding"));
            }
            pos += 4;
            let divergence_raw = read_i64(bytes, &mut pos)?;
            let conflict_raw = read_i64(bytes, &mut pos)?;
            let slash_accum_raw = read_i64(bytes, &mut pos)?;
            let update = match present {
                0 => {
                    if divergence_raw != 0 || conflict_raw != 0 || slash_accum_raw != 0 {
                        return Err(HostedError::InvalidLog(
                            "non-canonical absent update payload",
                        ));
                    }
                    None
                }
                1 => Some(CanonicalValidatorUpdate {
                    divergence_raw,
                    conflict_raw,
                    slash_accum_raw,
                }),
                _ => return Err(HostedError::InvalidLog("invalid update presence flag")),
            };
            updates.push(update);
        }
        let mut raw_txs = Vec::with_capacity(tx_count);
        for _ in 0..tx_count {
            let len = read_u32(bytes, &mut pos)? as usize;
            if len > MAX_RAW_TX_BYTES || pos + len > bytes.len() {
                return Err(HostedError::InvalidLog("invalid raw transaction length"));
            }
            raw_txs.push(bytes[pos..pos + len].to_vec());
            pos += len;
        }
        if pos != bytes.len() {
            return Err(HostedError::InvalidLog(
                "trailing bytes in canonical input record",
            ));
        }
        Ok(CanonicalInput {
            epoch,
            updates,
            raw_txs,
        })
    }

    fn read_u64(bytes: &[u8], pos: &mut usize) -> Result<u64, HostedError> {
        if *pos + 8 > bytes.len() {
            return Err(HostedError::InvalidLog("truncated u64"));
        }
        let mut out = [0u8; 8];
        out.copy_from_slice(&bytes[*pos..*pos + 8]);
        *pos += 8;
        Ok(u64::from_le_bytes(out))
    }

    fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, HostedError> {
        if *pos + 4 > bytes.len() {
            return Err(HostedError::InvalidLog("truncated u32"));
        }
        let mut out = [0u8; 4];
        out.copy_from_slice(&bytes[*pos..*pos + 4]);
        *pos += 4;
        Ok(u32::from_le_bytes(out))
    }

    fn read_i64(bytes: &[u8], pos: &mut usize) -> Result<i64, HostedError> {
        if *pos + 8 > bytes.len() {
            return Err(HostedError::InvalidLog("truncated i64"));
        }
        let mut out = [0u8; 8];
        out.copy_from_slice(&bytes[*pos..*pos + 8]);
        *pos += 8;
        Ok(i64::from_le_bytes(out))
    }
}
