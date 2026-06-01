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
pub mod smartcard {
    use sha3::{Digest, Sha3_256};
    use std::fmt;

    const IN_MEMORY_SIGNATURE_DOMAIN: &[u8] = b"QASH-DOMAIN-B-IN-MEMORY-SMARTCARD-SIGNATURE\0";

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TokenDescriptor {
        pub provider: &'static str,
        pub slot_id: u64,
        pub label: String,
        pub serial: String,
        pub mechanisms: Vec<&'static str>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SignRequest {
        pub key_label: String,
        pub payload: Vec<u8>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SmartcardError {
        TokenUnavailable,
        KeyNotFound,
        InvalidInput(&'static str),
        Provider(String),
    }

    impl fmt::Display for SmartcardError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SmartcardError::TokenUnavailable => write!(f, "token unavailable"),
                SmartcardError::KeyNotFound => write!(f, "key not found"),
                SmartcardError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
                SmartcardError::Provider(msg) => write!(f, "provider error: {msg}"),
            }
        }
    }

    pub trait KeyStore {
        fn token_descriptor(&self) -> Result<TokenDescriptor, SmartcardError>;
        fn sign(&self, req: &SignRequest) -> Result<Vec<u8>, SmartcardError>;
    }

    /// Minimal in-memory adapter that models a Domain-B token provider.
    ///
    /// This is not a PKCS#11 or hardware-token implementation. It gives hosted
    /// tests a fixed-size, domain-separated signing surface so PAL callers can
    /// exercise key selection and signature verification without embedding raw
    /// payload bytes into the signature artifact.
    #[derive(Debug, Clone)]
    pub struct InMemoryKeyStore {
        pub descriptor: TokenDescriptor,
        pub key_label: String,
        pub signing_key: [u8; 32],
    }

    impl InMemoryKeyStore {
        pub fn verify(&self, req: &SignRequest, signature: &[u8]) -> Result<(), SmartcardError> {
            let expected = self.sign(req)?;
            if signature == expected {
                Ok(())
            } else {
                Err(SmartcardError::Provider(
                    "signature verification failed".to_string(),
                ))
            }
        }
    }

    impl KeyStore for InMemoryKeyStore {
        fn token_descriptor(&self) -> Result<TokenDescriptor, SmartcardError> {
            Ok(self.descriptor.clone())
        }

        fn sign(&self, req: &SignRequest) -> Result<Vec<u8>, SmartcardError> {
            if req.payload.is_empty() {
                return Err(SmartcardError::InvalidInput("payload cannot be empty"));
            }
            if req.key_label != self.key_label {
                return Err(SmartcardError::KeyNotFound);
            }
            let mut hasher = Sha3_256::new();
            hasher.update(IN_MEMORY_SIGNATURE_DOMAIN);
            update_len_prefixed(&mut hasher, self.descriptor.provider.as_bytes());
            hasher.update(self.descriptor.slot_id.to_le_bytes());
            update_len_prefixed(&mut hasher, self.descriptor.label.as_bytes());
            update_len_prefixed(&mut hasher, self.descriptor.serial.as_bytes());
            update_len_prefixed(&mut hasher, req.key_label.as_bytes());
            hasher.update(self.signing_key);
            update_len_prefixed(&mut hasher, &req.payload);
            Ok(hasher.finalize().to_vec())
        }
    }

    fn update_len_prefixed(hasher: &mut Sha3_256, bytes: &[u8]) {
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
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
    use qash_consensus::{
        advance_epoch, advance_epoch_sharded, validate_zk_profile, EpochInput, EpochShardingInput,
        EpochState, FixedPoint, HaltReason, PublicTranscript, ShardCommitment, TransitionResult,
        ValidatorUpdate, ZkProfile, MAX_VALIDATORS,
    };
    use std::collections::VecDeque;
    use std::fs::{File, OpenOptions};
    use std::io::{self, ErrorKind, Read, Write};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use zeroize::Zeroize;

    const LOG_MAGIC: &[u8; 8] = b"QPALOG1\0";
    const RECORD_MAGIC: &[u8; 8] = b"QPAIN1\0\0";
    const MAX_RAW_TX_BYTES: usize = 1 << 20;
    const COMMITMENT_FRAME_MAGIC: &[u8; 8] = b"QP2PCOM\0";
    const COMMITMENT_FRAME_BYTES: usize = 8 + 8 + 32 + 32 + 32 + 48 + 256;

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

    /// Domain-B operational halt preparation.
    ///
    /// This records the PAL-side actions that precede the deployed
    /// non-returning halt loop. Domain A halt state remains replayable and is
    /// not mutated by these platform operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PreparedHalt {
        pub reason: HaltReason,
        pub critical_memory_zeroized: bool,
        pub scheduler_disable_requested: bool,
        pub watchdog_reset_requested: bool,
    }

    /// Domain-B attestation report. It is verified before trust decisions, but
    /// never feeds directly into Domain-A state transition inputs.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AttestationReport {
        pub validator_id: [u8; 48],
        pub quote: [u8; 256],
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AttestationVerdict {
        Trusted,
        Rejected,
    }

    pub trait AttestationVerifier {
        fn verify(&self, report: &AttestationReport) -> AttestationVerdict;
    }

    /// Deterministic test verifier. Production Linux TPM verification should
    /// implement the same trait without changing Domain-A admission semantics.
    #[derive(Debug, Clone, Copy)]
    pub struct StaticAttestationVerifier {
        pub trusted_quote: [u8; 256],
    }

    impl AttestationVerifier for StaticAttestationVerifier {
        fn verify(&self, report: &AttestationReport) -> AttestationVerdict {
            if report.quote == self.trusted_quote {
                AttestationVerdict::Trusted
            } else {
                AttestationVerdict::Rejected
            }
        }
    }

    /// Commitment-only network frame. It intentionally excludes raw
    /// transaction bytes and host timing metadata.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CommitmentFrame {
        pub epoch: u64,
        pub state_root: [u8; 32],
        pub receipt_root: [u8; 32],
        pub efb_root: [u8; 32],
        pub validator_id: [u8; 48],
        pub attestation_quote: [u8; 256],
    }

    pub trait CommitmentTransport {
        fn send_commitment(&mut self, frame: &CommitmentFrame) -> Result<(), HostedError>;
        fn recv_commitment(&mut self) -> Result<Option<CommitmentFrame>, HostedError>;
    }

    #[derive(Debug, Clone, Default)]
    pub struct InMemoryCommitmentTransport {
        queue: VecDeque<Vec<u8>>,
    }

    /// A consensus-admissible input record after Domain B has normalized away
    /// transport, timing, and host metadata.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CanonicalInput {
        pub epoch: u64,
        pub updates: Vec<Option<CanonicalValidatorUpdate>>,
        pub raw_txs: Vec<Vec<u8>>,
        pub sharding: Option<CanonicalShardingInput>,
    }

    /// Canonical fixed-point validator metric update.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CanonicalValidatorUpdate {
        pub divergence_raw: i64,
        pub conflict_raw: i64,
        pub slash_accum_raw: i64,
    }

    /// Canonical sharded epoch data admitted by Domain B.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CanonicalShardingInput {
        pub shard_commitments: Vec<CanonicalShardCommitment>,
        pub zk_batch_root: [u8; 32],
        pub zk_profile: Option<CanonicalZkProfile>,
    }

    /// Canonical fixed-width shard commitment.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CanonicalShardCommitment {
        pub shard_id: u32,
        pub state_root: [u8; 32],
        pub receipt_root: [u8; 32],
    }

    /// Domain-B proof profile metadata. Proof bytes remain in Domain B; Domain
    /// A sees only the validated profile and `zk_batch_root` commitment.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CanonicalZkProfile {
        pub profile_id: u32,
        pub recursion_depth: u8,
        pub layer1_aggregation_factor: u16,
    }

    /// PAL-side proof bundle skeleton for the PR #93 two-layer STARK path.
    /// This is not a verifier; it defines the hosted boundary that a real
    /// Plonky3/FRI backend must implement without feeding proof bytes into
    /// Domain A.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ZkProofBundle {
        pub profile: CanonicalZkProfile,
        pub shard_proof_count: u32,
        pub aggregation_proof_count: u32,
        pub batch_root: [u8; 32],
    }

    pub trait ZkProofVerifier {
        fn verify_bundle(&self, bundle: &ZkProofBundle) -> Result<[u8; 32], HostedError>;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct StaticZkProofVerifier {
        pub accepted_profile: CanonicalZkProfile,
        pub accepted_batch_root: [u8; 32],
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
                sharding: None,
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
                updates[idx] = update.as_ref().map(|u| ValidatorUpdate {
                    divergence_new: FixedPoint::from_raw(i128::from(u.divergence_raw)),
                    conflict_new: FixedPoint::from_raw(i128::from(u.conflict_raw)),
                    slash_accum_new: FixedPoint::from_raw(i128::from(u.slash_accum_raw)),
                });
            }

            Ok(EpochInput {
                updates,
                update_count: state.validator_count,
                protocol_version: if self.sharding.is_some() {
                    qash_consensus::envelope::PROTOCOL_VERSION_V1_2
                } else {
                    qash_consensus::envelope::PROTOCOL_VERSION_V1_1
                },
            })
        }
    }

    impl CanonicalShardingInput {
        fn to_epoch_sharding_input(&self) -> Result<Vec<ShardCommitment>, HostedError> {
            if self.shard_commitments.is_empty() {
                return Err(HostedError::InvalidInput(
                    "sharded input has no shard commitments",
                ));
            }
            let mut out = Vec::with_capacity(self.shard_commitments.len());
            for shard in &self.shard_commitments {
                out.push(ShardCommitment {
                    shard_id: shard.shard_id,
                    state_root: shard.state_root,
                    receipt_root: shard.receipt_root,
                });
            }
            if let Some(profile) = self.zk_profile {
                validate_zk_profile(&profile.into_consensus())
                    .map_err(|_| HostedError::InvalidInput("invalid ZK profile"))?;
            }
            Ok(out)
        }
    }

    impl CanonicalZkProfile {
        pub fn pr93_plonky3_fri_poseidon_qash() -> Self {
            let profile = ZkProfile::PLONKY3_FRI_POSEIDON_QASH;
            Self {
                profile_id: profile.profile_id,
                recursion_depth: profile.recursion_depth,
                layer1_aggregation_factor: profile.layer1_aggregation_factor,
            }
        }

        fn into_consensus(self) -> ZkProfile {
            ZkProfile {
                profile_id: self.profile_id,
                recursion_depth: self.recursion_depth,
                layer1_aggregation_factor: self.layer1_aggregation_factor,
            }
        }
    }

    impl ZkProofVerifier for StaticZkProofVerifier {
        fn verify_bundle(&self, bundle: &ZkProofBundle) -> Result<[u8; 32], HostedError> {
            if bundle.profile != self.accepted_profile {
                return Err(HostedError::InvalidInput("unexpected ZK profile"));
            }
            validate_zk_profile(&bundle.profile.into_consensus())
                .map_err(|_| HostedError::InvalidInput("invalid ZK profile"))?;
            if bundle.batch_root != self.accepted_batch_root {
                return Err(HostedError::InvalidInput("unexpected ZK batch root"));
            }
            if bundle.shard_proof_count == 0 || bundle.aggregation_proof_count == 0 {
                return Err(HostedError::InvalidInput("empty ZK proof bundle"));
            }
            Ok(bundle.batch_root)
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
            let epoch_input = input.to_epoch_input(state)?;
            let raw_refs: Vec<&[u8]> = input.raw_txs.iter().map(Vec::as_slice).collect();

            // Run the deterministic transition on a scratch copy first.  The
            // accepted record is made durable before publishing the resulting
            // Domain A state to the caller, so an append failure cannot leave
            // in-memory state ahead of the recoverable log.
            let mut next_state = *state;
            let result = match &input.sharding {
                Some(sharding) => {
                    let shards = sharding.to_epoch_sharding_input()?;
                    let epoch_sharding = EpochShardingInput {
                        shard_commitments: &shards,
                        zk_batch_root: sharding.zk_batch_root,
                    };
                    advance_epoch_sharded(&mut next_state, &epoch_input, &raw_refs, &epoch_sharding)
                        .map_err(HostedError::ConsensusHalt)?
                }
                None => advance_epoch(&mut next_state, &epoch_input, &raw_refs)
                    .map_err(HostedError::ConsensusHalt)?,
            };
            append_record(&self.log_path, input)?;
            *state = next_state;
            Ok(result)
        }

        /// Replay the persisted canonical log from a supplied genesis state.
        pub fn replay_from_genesis(&self, genesis: EpochState) -> Result<EpochState, HostedError> {
            let mut state = genesis;
            for input in read_records(&self.log_path)? {
                let epoch_input = input.to_epoch_input(&state)?;
                let raw_refs: Vec<&[u8]> = input.raw_txs.iter().map(Vec::as_slice).collect();
                match &input.sharding {
                    Some(sharding) => {
                        let shards = sharding.to_epoch_sharding_input()?;
                        let epoch_sharding = EpochShardingInput {
                            shard_commitments: &shards,
                            zk_batch_root: sharding.zk_batch_root,
                        };
                        advance_epoch_sharded(&mut state, &epoch_input, &raw_refs, &epoch_sharding)
                            .map_err(HostedError::ConsensusHalt)?;
                    }
                    None => {
                        advance_epoch(&mut state, &epoch_input, &raw_refs)
                            .map_err(HostedError::ConsensusHalt)?;
                    }
                }
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

        /// Prepare Domain-B operational halt behavior without touching
        /// Domain-A state. The non-returning step is deliberately separate so
        /// tests can verify zeroization/watchdog ownership without diverging.
        pub fn prepare_absorbing_halt(
            &mut self,
            critical_memory: &mut [u8],
            reason: HaltReason,
        ) -> PreparedHalt {
            critical_memory.zeroize();
            self.request_reset();
            PreparedHalt {
                reason,
                critical_memory_zeroized: critical_memory.iter().all(|b| *b == 0),
                scheduler_disable_requested: true,
                watchdog_reset_requested: true,
            }
        }
    }

    impl PreparedHalt {
        /// Enter the deployed PAL absorbing halt loop.
        pub fn enter_non_returning_loop(self) -> ! {
            let _ = self;
            loop {
                std::hint::spin_loop();
            }
        }
    }

    impl CommitmentFrame {
        pub fn from_transcript(
            transcript: &PublicTranscript,
            validator_id: [u8; 48],
            attestation_quote: [u8; 256],
        ) -> Self {
            CommitmentFrame {
                epoch: transcript.epoch,
                state_root: transcript.state_root,
                receipt_root: transcript.receipt_root,
                efb_root: transcript.efb_root,
                validator_id,
                attestation_quote,
            }
        }
    }

    // ── All-of attestation transcript helpers ─────────────────────────────────

    /// Compute an all-of dual-root pair for a `CommitmentFrame` attestation transcript.
    ///
    /// Canonical transcript: `epoch_le || state_root || receipt_root || efb_root ||
    /// attestation_quote`. Salt is `validator_id`. Not FIPS/CAVP/ACVP evidence.
    /// Does NOT certify the underlying TPM/TEE/HSM backend correctness — it
    /// hardens QASH's transcript binding only.
    pub fn compute_attestation_transcript_root_pair(
        frame: &CommitmentFrame,
    ) -> crate::crypto::dual_hash::AllOfHashPair32 {
        let data = attestation_transcript_data(frame);
        crate::crypto::dual_hash::allof_hash_pair_32(
            b"qash-attestation-transcript-v1",
            &frame.validator_id,
            &data,
        )
    }

    /// Verify an all-of dual-root pair against a fresh computation for the given frame.
    ///
    /// Returns `true` only when both SHA3 and BLAKE3 roots match independently.
    pub fn verify_attestation_transcript_root_pair(
        frame: &CommitmentFrame,
        pair: &crate::crypto::dual_hash::AllOfHashPair32,
    ) -> bool {
        let data = attestation_transcript_data(frame);
        crate::crypto::dual_hash::verify_allof_hash_pair_32(
            pair,
            b"qash-attestation-transcript-v1",
            &frame.validator_id,
            &data,
        )
    }

    fn attestation_transcript_data(frame: &CommitmentFrame) -> [u8; 360] {
        let mut data = [0u8; 360];
        data[..8].copy_from_slice(&frame.epoch.to_le_bytes());
        data[8..40].copy_from_slice(&frame.state_root);
        data[40..72].copy_from_slice(&frame.receipt_root);
        data[72..104].copy_from_slice(&frame.efb_root);
        data[104..360].copy_from_slice(&frame.attestation_quote);
        data
    }

    impl InMemoryCommitmentTransport {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl CommitmentTransport for InMemoryCommitmentTransport {
        fn send_commitment(&mut self, frame: &CommitmentFrame) -> Result<(), HostedError> {
            self.queue.push_back(encode_commitment_frame(frame));
            Ok(())
        }

        fn recv_commitment(&mut self) -> Result<Option<CommitmentFrame>, HostedError> {
            match self.queue.pop_front() {
                Some(bytes) => decode_commitment_frame(&bytes).map(Some),
                None => Ok(None),
            }
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

    fn encode_commitment_frame(frame: &CommitmentFrame) -> Vec<u8> {
        let mut out = Vec::with_capacity(COMMITMENT_FRAME_BYTES);
        out.extend_from_slice(COMMITMENT_FRAME_MAGIC);
        out.extend_from_slice(&frame.epoch.to_le_bytes());
        out.extend_from_slice(&frame.state_root);
        out.extend_from_slice(&frame.receipt_root);
        out.extend_from_slice(&frame.efb_root);
        out.extend_from_slice(&frame.validator_id);
        out.extend_from_slice(&frame.attestation_quote);
        out
    }

    fn decode_commitment_frame(bytes: &[u8]) -> Result<CommitmentFrame, HostedError> {
        if bytes.len() != COMMITMENT_FRAME_BYTES {
            return Err(HostedError::InvalidLog("invalid commitment frame length"));
        }
        if &bytes[..8] != COMMITMENT_FRAME_MAGIC {
            return Err(HostedError::InvalidLog("invalid commitment frame magic"));
        }
        let mut pos = 8;
        let epoch = read_u64(bytes, &mut pos)?;
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;
        let mut receipt_root = [0u8; 32];
        receipt_root.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;
        let mut efb_root = [0u8; 32];
        efb_root.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;
        let mut validator_id = [0u8; 48];
        validator_id.copy_from_slice(&bytes[pos..pos + 48]);
        pos += 48;
        let mut attestation_quote = [0u8; 256];
        attestation_quote.copy_from_slice(&bytes[pos..pos + 256]);
        Ok(CommitmentFrame {
            epoch,
            state_root,
            receipt_root,
            efb_root,
            validator_id,
            attestation_quote,
        })
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
        match &input.sharding {
            Some(sharding) => {
                if sharding.shard_commitments.len() > qash_consensus::MAX_SHARDS {
                    return Err(HostedError::InvalidInput("too many shard commitments"));
                }
                out.push(1);
                out.extend_from_slice(&[0u8; 3]);
                out.extend_from_slice(&sharding.zk_batch_root);
                match sharding.zk_profile {
                    Some(profile) => {
                        out.push(1);
                        out.extend_from_slice(&[0u8; 3]);
                        out.extend_from_slice(&profile.profile_id.to_le_bytes());
                        out.push(profile.recursion_depth);
                        out.push(0);
                        out.extend_from_slice(&profile.layer1_aggregation_factor.to_le_bytes());
                    }
                    None => {
                        out.push(0);
                        out.extend_from_slice(&[0u8; 11]);
                    }
                }
                out.extend_from_slice(&(sharding.shard_commitments.len() as u32).to_le_bytes());
                for shard in &sharding.shard_commitments {
                    out.extend_from_slice(&shard.shard_id.to_le_bytes());
                    out.extend_from_slice(&shard.state_root);
                    out.extend_from_slice(&shard.receipt_root);
                }
            }
            None => {
                out.push(0);
                out.extend_from_slice(&[0u8; 3]);
            }
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
            if pos + 4 > bytes.len() {
                return Err(HostedError::InvalidLog(
                    "truncated canonical sharding presence flag",
                ));
            }
            let sharding_present = bytes[pos];
            if bytes[pos + 1] != 0 || bytes[pos + 2] != 0 || bytes[pos + 3] != 0 {
                return Err(HostedError::InvalidLog("non-canonical sharding padding"));
            }
            pos += 4;
            let sharding = match sharding_present {
                0 => None,
                1 => {
                    if pos + 36 > bytes.len() {
                        return Err(HostedError::InvalidLog("truncated sharding header"));
                    }
                    let mut zk_batch_root = [0u8; 32];
                    zk_batch_root.copy_from_slice(&bytes[pos..pos + 32]);
                    pos += 32;
                    if pos + 12 > bytes.len() {
                        return Err(HostedError::InvalidLog("truncated ZK profile"));
                    }
                    let profile_present = bytes[pos];
                    if bytes[pos + 1] != 0 || bytes[pos + 2] != 0 || bytes[pos + 3] != 0 {
                        return Err(HostedError::InvalidLog("non-canonical ZK profile padding"));
                    }
                    pos += 4;
                    let zk_profile = match profile_present {
                        0 => {
                            if bytes[pos..pos + 8] != [0u8; 8] {
                                return Err(HostedError::InvalidLog(
                                    "non-canonical absent ZK profile payload",
                                ));
                            }
                            pos += 8;
                            None
                        }
                        1 => {
                            let profile_id = read_u32(bytes, &mut pos)?;
                            let recursion_depth = bytes[pos];
                            if bytes[pos + 1] != 0 {
                                return Err(HostedError::InvalidLog(
                                    "non-canonical ZK recursion padding",
                                ));
                            }
                            pos += 2;
                            let mut factor = [0u8; 2];
                            factor.copy_from_slice(&bytes[pos..pos + 2]);
                            pos += 2;
                            let profile = CanonicalZkProfile {
                                profile_id,
                                recursion_depth,
                                layer1_aggregation_factor: u16::from_le_bytes(factor),
                            };
                            validate_zk_profile(&profile.into_consensus())
                                .map_err(|_| HostedError::InvalidLog("invalid ZK profile"))?;
                            Some(profile)
                        }
                        _ => return Err(HostedError::InvalidLog("invalid ZK profile flag")),
                    };
                    let shard_count = read_u32(bytes, &mut pos)? as usize;
                    if shard_count == 0 || shard_count > qash_consensus::MAX_SHARDS {
                        return Err(HostedError::InvalidLog("invalid shard commitment count"));
                    }
                    let mut shard_commitments = Vec::with_capacity(shard_count);
                    for _ in 0..shard_count {
                        let shard_id = read_u32(bytes, &mut pos)?;
                        if pos + 64 > bytes.len() {
                            return Err(HostedError::InvalidLog("truncated shard commitment"));
                        }
                        let mut state_root = [0u8; 32];
                        state_root.copy_from_slice(&bytes[pos..pos + 32]);
                        pos += 32;
                        let mut receipt_root = [0u8; 32];
                        receipt_root.copy_from_slice(&bytes[pos..pos + 32]);
                        pos += 32;
                        shard_commitments.push(CanonicalShardCommitment {
                            shard_id,
                            state_root,
                            receipt_root,
                        });
                    }
                    Some(CanonicalShardingInput {
                        shard_commitments,
                        zk_batch_root,
                        zk_profile,
                    })
                }
                _ => return Err(HostedError::InvalidLog("invalid sharding presence flag")),
            };
            if pos != bytes.len() {
                return Err(HostedError::InvalidLog(
                    "trailing bytes in canonical input record",
                ));
            }
            return Ok(CanonicalInput {
                epoch,
                updates,
                raw_txs,
                sharding,
            });
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
            sharding: None,
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};

        fn tmp_log(label: &str) -> std::path::PathBuf {
            use std::sync::atomic::{AtomicU64, Ordering};
            static CTR: AtomicU64 = AtomicU64::new(0);
            let n = CTR.fetch_add(1, Ordering::Relaxed);
            std::env::temp_dir().join(format!(
                "qash-pal-{label}-{}-{n}.log",
                std::process::id()
            ))
        }

        fn test_genesis(validator_count: u32) -> EpochState {
            let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
            for i in 0..validator_count as usize {
                validator_ids[i][0..4].copy_from_slice(&(i as u32).to_le_bytes());
            }
            EpochState {
                epoch: 0,
                halt_reason: HaltReason::None,
                entropy_seed: [0u8; 32],
                validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
                validator_count,
                convergence_window: ConvergenceWindow::new(),
                nonces: [0u64; MAX_VALIDATORS],
                validator_ids,
                cascade_health: 0,
                causal_fingerprint: [0u8; 32],
                state_root: [0u8; 32],
                receipt_root: [0u8; 32],
                efb_root: [0u8; 32],
            }
        }

        // --- Host lifecycle ---

        #[test]
        fn host_new_creates_empty_state() {
            let path = tmp_log("t");
            let host = Host::new(&path).expect("Host::new");
            assert_eq!(host.attestation_quote(), [0u8; 256]);
            assert!(!host.reset_requested());
            assert_eq!(host.sent_frames(), &[] as &[Vec<u8>]);
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn host_apply_canonical_input_advances_epoch() {
            let path = tmp_log("t");
            let mut host = Host::new(&path).expect("Host::new");
            let mut state = test_genesis(4);
            assert_eq!(state.epoch, 0);

            let input = CanonicalInput::idle(0, 4).unwrap();
            host.apply_canonical_input(&mut state, &input).expect("advance");
            assert_eq!(state.epoch, 1);
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn host_replay_from_genesis_reproduces_state() {
            let path = tmp_log("t");
            let mut host = Host::new(&path).expect("Host::new");
            let genesis_state = test_genesis(4);
            let mut state = genesis_state;

            let input = CanonicalInput::idle(0, 4).unwrap();
            host.apply_canonical_input(&mut state, &input).expect("advance");
            let root_after_one = state.state_root;

            let replayed = host.replay_from_genesis(genesis_state).expect("replay");
            assert_eq!(replayed.state_root, root_after_one);
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn host_epoch_mismatch_is_rejected() {
            let path = tmp_log("t");
            let mut host = Host::new(&path).expect("Host::new");
            let mut state = test_genesis(4);

            let wrong_epoch_input = CanonicalInput::idle(99, 4).unwrap();
            assert!(matches!(
                host.apply_canonical_input(&mut state, &wrong_epoch_input),
                Err(HostedError::InvalidInput(_))
            ));
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn host_network_frame_roundtrip() {
            let path = tmp_log("t");
            let mut host = Host::new(&path).expect("Host::new");

            assert!(host.recv_network_frame().is_none());
            host.enqueue_network_frame(vec![1, 2, 3]);
            host.enqueue_network_frame(vec![4, 5]);
            assert_eq!(host.recv_network_frame(), Some(vec![1, 2, 3]));
            assert_eq!(host.recv_network_frame(), Some(vec![4, 5]));
            assert!(host.recv_network_frame().is_none());
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn host_sent_frames_accumulate() {
            let path = tmp_log("t");
            let mut host = Host::new(&path).expect("Host::new");
            host.send_network_frame(b"hello");
            host.send_network_frame(b"world");
            assert_eq!(host.sent_frames(), &[b"hello".to_vec(), b"world".to_vec()]);
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn host_attestation_quote_roundtrip() {
            let path = tmp_log("t");
            let mut host = Host::new(&path).expect("Host::new");
            let quote = [0xABu8; 256];
            host.set_attestation_quote(quote);
            assert_eq!(host.attestation_quote(), quote);
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn host_prepare_absorbing_halt_zeroes_memory_and_sets_reset() {
            let path = tmp_log("t");
            let mut host = Host::new(&path).expect("Host::new");
            let mut critical = [0xFFu8; 64];
            let prepared = host.prepare_absorbing_halt(&mut critical, HaltReason::None);
            assert!(critical.iter().all(|b| *b == 0), "critical memory not zeroed");
            assert!(host.reset_requested());
            assert!(prepared.critical_memory_zeroized);
            assert!(prepared.scheduler_disable_requested);
            assert!(prepared.watchdog_reset_requested);
            let _ = std::fs::remove_file(&path);
        }

        // --- InMemoryCommitmentTransport ---

        #[test]
        fn commitment_transport_send_recv_roundtrip() {
            let mut transport = InMemoryCommitmentTransport::new();
            assert!(transport.recv_commitment().unwrap().is_none());

            let frame = CommitmentFrame {
                epoch: 7,
                state_root: [0x11u8; 32],
                receipt_root: [0x22u8; 32],
                efb_root: [0x33u8; 32],
                validator_id: [0x44u8; 48],
                attestation_quote: [0x55u8; 256],
            };
            transport.send_commitment(&frame).unwrap();
            let received = transport.recv_commitment().unwrap().expect("frame");
            assert_eq!(received.epoch, 7);
            assert_eq!(received.state_root, [0x11u8; 32]);
            assert_eq!(received.validator_id, [0x44u8; 48]);
        }

        #[test]
        fn commitment_transport_empty_after_recv() {
            let mut transport = InMemoryCommitmentTransport::new();
            let frame = CommitmentFrame {
                epoch: 1,
                state_root: [0u8; 32],
                receipt_root: [0u8; 32],
                efb_root: [0u8; 32],
                validator_id: [0u8; 48],
                attestation_quote: [0u8; 256],
            };
            transport.send_commitment(&frame).unwrap();
            let _ = transport.recv_commitment().unwrap();
            assert!(transport.recv_commitment().unwrap().is_none());
        }

        // --- StaticZkProofVerifier ---

        #[test]
        fn zk_verifier_accepts_matching_bundle() {
            let profile = CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash();
            let batch_root = [0xCAu8; 32];
            let verifier = StaticZkProofVerifier {
                accepted_profile: profile.clone(),
                accepted_batch_root: batch_root,
            };
            let bundle = ZkProofBundle {
                profile,
                shard_proof_count: 4,
                aggregation_proof_count: 1,
                batch_root,
            };
            assert_eq!(verifier.verify_bundle(&bundle).unwrap(), batch_root);
        }

        #[test]
        fn zk_verifier_rejects_wrong_profile() {
            let profile = CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash();
            let wrong_profile = CanonicalZkProfile {
                profile_id: 0xFF,
                recursion_depth: 0,
                layer1_aggregation_factor: 0,
            };
            let batch_root = [0x01u8; 32];
            let verifier = StaticZkProofVerifier {
                accepted_profile: profile,
                accepted_batch_root: batch_root,
            };
            let bundle = ZkProofBundle {
                profile: wrong_profile,
                shard_proof_count: 1,
                aggregation_proof_count: 1,
                batch_root,
            };
            assert!(matches!(
                verifier.verify_bundle(&bundle),
                Err(HostedError::InvalidInput(_))
            ));
        }

        #[test]
        fn zk_verifier_rejects_wrong_batch_root() {
            let profile = CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash();
            let batch_root = [0x01u8; 32];
            let verifier = StaticZkProofVerifier {
                accepted_profile: profile.clone(),
                accepted_batch_root: batch_root,
            };
            let bundle = ZkProofBundle {
                profile,
                shard_proof_count: 1,
                aggregation_proof_count: 1,
                batch_root: [0x02u8; 32],
            };
            assert!(matches!(
                verifier.verify_bundle(&bundle),
                Err(HostedError::InvalidInput(_))
            ));
        }

        #[test]
        fn zk_verifier_rejects_empty_proof_counts() {
            let profile = CanonicalZkProfile::pr93_plonky3_fri_poseidon_qash();
            let batch_root = [0x01u8; 32];
            let verifier = StaticZkProofVerifier {
                accepted_profile: profile.clone(),
                accepted_batch_root: batch_root,
            };
            let bundle = ZkProofBundle {
                profile,
                shard_proof_count: 0,
                aggregation_proof_count: 1,
                batch_root,
            };
            assert!(matches!(
                verifier.verify_bundle(&bundle),
                Err(HostedError::InvalidInput(_))
            ));
        }

        // --- Attestation transcript all-of roots ---

        fn sample_frame() -> CommitmentFrame {
            CommitmentFrame {
                epoch: 42,
                state_root: [0x11u8; 32],
                receipt_root: [0x22u8; 32],
                efb_root: [0x33u8; 32],
                validator_id: [0x44u8; 48],
                attestation_quote: [0x55u8; 256],
            }
        }

        #[test]
        fn attestation_transcript_accepts_exact_root_pair() {
            let frame = sample_frame();
            let pair = compute_attestation_transcript_root_pair(&frame);
            assert!(verify_attestation_transcript_root_pair(&frame, &pair));
        }

        #[test]
        fn attestation_transcript_rejects_modified_sha3_root() {
            let frame = sample_frame();
            let mut pair = compute_attestation_transcript_root_pair(&frame);
            pair.sha3_512_32[0] ^= 0xFF;
            assert!(!verify_attestation_transcript_root_pair(&frame, &pair));
        }

        #[test]
        fn attestation_transcript_rejects_modified_blake3_root() {
            let frame = sample_frame();
            let mut pair = compute_attestation_transcript_root_pair(&frame);
            pair.blake3_32[0] ^= 0xFF;
            assert!(!verify_attestation_transcript_root_pair(&frame, &pair));
        }

        #[test]
        fn attestation_transcript_root_changes_when_nonce_changes() {
            // "nonce" = epoch — temporal binding
            let mut frame2 = sample_frame();
            frame2.epoch = 99;
            let pair1 = compute_attestation_transcript_root_pair(&sample_frame());
            let pair2 = compute_attestation_transcript_root_pair(&frame2);
            assert_ne!(pair1.sha3_512_32, pair2.sha3_512_32);
            assert_ne!(pair1.blake3_32, pair2.blake3_32);
        }

        #[test]
        fn attestation_transcript_root_changes_when_measurement_changes() {
            // "measurement" = attestation_quote (TPM/TEE quote bytes)
            let mut frame2 = sample_frame();
            frame2.attestation_quote = [0xAAu8; 256];
            let pair1 = compute_attestation_transcript_root_pair(&sample_frame());
            let pair2 = compute_attestation_transcript_root_pair(&frame2);
            assert_ne!(pair1.sha3_512_32, pair2.sha3_512_32);
            assert_ne!(pair1.blake3_32, pair2.blake3_32);
        }
    }
}
