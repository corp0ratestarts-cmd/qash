//! Domain A / Domain B type-level boundary enforcement.
//!
//! # Two-domain model
//!
//! QASH partitions execution into two domains:
//!
//! - **Domain A** (`qash-consensus`): deterministic consensus core. No `unsafe`,
//!   no `f32`/`f64`, no `HashMap`, no wall-clock, no OS entropy. Replay-invariant
//!   across all authorized ISAs.
//!
//! - **Domain B** (`qash-pal` + hosted binary): operational / PAL layer.
//!   Non-determinism and `unsafe` are permitted here, but Domain B values must
//!   never flow into Domain A computations unmediated.
//!
//! # Boundary types in this module
//!
//! - [`DomainA`] — marker trait for types that belong to Domain A. Implementing
//!   this trait asserts the implementor satisfies all Domain A constraints.
//!
//! - [`CapToken<T>`] — capability token wrapping a Domain B value. Domain A
//!   functions must never accept a `CapToken<T>` as a plain `T`; the value must
//!   be explicitly unwrapped via [`CapToken::into_inner`] at the PAL boundary,
//!   signalling an intentional cross-domain data flow.

/// Marker trait for types that are safe for use in Domain A (consensus core).
///
/// Implementing this trait is a declaration that the type satisfies:
/// - No floating-point fields or arithmetic
/// - No `unsafe` in its implementation
/// - Deterministic encoding across all authorized ISAs
/// - All arithmetic is checked (overflow → `HaltReason::*`)
///
/// The trait has no required methods; it is purely documentary / type-level.
pub trait DomainA: private::Sealed {}

mod private {
    pub trait Sealed {}
}

/// Capability token wrapping a Domain B value `T`.
///
/// A `CapToken<T>` marks `T` as having originated in Domain B (the PAL /
/// operational layer). Domain A computation paths must never accept a plain
/// `T` that actually came from Domain B; wrapping it in `CapToken` makes the
/// provenance explicit and forces an intentional unwrap at the boundary.
///
/// # Usage pattern
///
/// ```text
/// // Domain B (PAL):
/// let token: CapToken<u64> = CapToken::new(os_timestamp());
///
/// // Domain A boundary admission point:
/// let epoch_hint: u64 = token.into_inner();
/// // caller is now responsible for proving this value is safe to use in DA
/// ```
///
/// This is a v1.1 stub. Full enforcement is achieved by:
/// 1. Clippy `disallowed_types` rules (see `crates/consensus/clippy.toml`)
/// 2. The `DomainA` marker trait on every Domain A state type
/// 3. Audit: search for any `CapToken::into_inner` call sites outside PAL
#[repr(transparent)]
pub struct CapToken<T>(T);

impl<T> CapToken<T> {
    /// Wrap a Domain B value in a capability token.
    ///
    /// Call this only from Domain B (PAL) code at the explicit boundary.
    /// Never call this inside `qash-consensus`.
    pub fn new(val: T) -> Self {
        CapToken(val)
    }

    /// Unwrap the capability token, yielding the inner Domain B value.
    ///
    /// The caller asserts that:
    /// - This unwrap happens at a documented PAL boundary admission point.
    /// - The resulting value will not introduce non-determinism into any
    ///   Domain A computation.
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Borrow the inner value without consuming the token.
    pub fn as_inner(&self) -> &T {
        &self.0
    }
}

// ------------------------------------------------------------------
// DomainA implementations for consensus core types
// ------------------------------------------------------------------
// These are the canonical Domain A types. Every type listed here has been
// audited to satisfy the Domain A constraints in CLAUDE.md.

use crate::fixed_point::FixedPoint;
use crate::hash::DomainTag;
use crate::lyapunov::ValidatorMetrics;
use crate::transition::EpochState;

impl private::Sealed for FixedPoint {}
impl DomainA for FixedPoint {}

impl private::Sealed for ValidatorMetrics {}
impl DomainA for ValidatorMetrics {}

impl private::Sealed for EpochState {}
impl DomainA for EpochState {}

impl private::Sealed for DomainTag {}
impl DomainA for DomainTag {}
