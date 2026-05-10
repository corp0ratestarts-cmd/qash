#![no_std]

pub trait AccelerationBackend {
    fn accelerate_hash_cascade(
        &self,
        input: &[u8],
        domain: &[u8],
        output: &mut [u8; 32],
    ) -> Result<(), AccelerationError>;

    fn accelerate_field_ops(
        &self,
        _a: &[u8; 32],
        _b: &[u8; 32],
        _prime: &[u8; 32],
        _op: FieldOp,
    ) -> Result<[u8; 32], AccelerationError> {
        Err(AccelerationError::NotImplemented)
    }

    fn platform_measurement(&self) -> [u8; 32];
}

pub enum AccelerationError {
    NotImplemented,
    InvalidInput,
    VerificationFailed,
}

pub enum FieldOp {
    Add, Mul, Mod,
}
