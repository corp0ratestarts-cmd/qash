/// ZK proof verifier backends (Domain B only).
///
/// Proof bytes are processed entirely within Domain B. The only value
/// that crosses into Domain A is the 32-byte batch_root extracted after
/// successful verification — never raw proof bytes or intermediate state.
pub mod plonky3;
