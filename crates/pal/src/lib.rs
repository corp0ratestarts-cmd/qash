/// Platform Abstraction Layer (PAL) traits.
pub trait Time { fn epoch_counter() -> u64; }
pub trait Net { fn send(_: &[u8]); fn recv(_: &mut [u8]) -> usize; }
pub trait Attest { fn tpm_quote() -> [u8; 256]; }
pub trait Halt { fn absorbing_reset() -> !; }

#[cfg(feature = "std")]
pub mod hosted {
    use super::*;

    pub struct Host;

    impl Time for Host {
        fn epoch_counter() -> u64 { 0 }
    }

    impl Net for Host {
        fn send(_: &[u8]) {}
        fn recv(_: &mut [u8]) -> usize { 0 }
    }

    impl Attest for Host {
        fn tpm_quote() -> [u8; 256] { [0u8; 256] }
    }

    impl Halt for Host {
        fn absorbing_reset() -> ! { loop { } }
    }
}
