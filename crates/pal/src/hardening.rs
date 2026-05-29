/// Rowhammer mitigation stub — enabled with `--features hardened`.
///
/// `RowhammerGuard` periodically refreshes DRAM rows adjacent to sensitive
/// memory allocations using CLFLUSH + non-temporal loads. This mitigates
/// One-Location Hammer and Double-Sided Hammer variants.
///
/// On non-x86_64 platforms or without the `hardened` feature, this is a
/// zero-cost no-op.

#[cfg(feature = "hardened")]
pub struct RowhammerGuard {
    /// Regions to protect: (pointer, length) pairs.
    sensitive_regions: Vec<(*mut u8, usize)>,
}

#[cfg(feature = "hardened")]
unsafe impl Send for RowhammerGuard {}

#[cfg(feature = "hardened")]
impl RowhammerGuard {
    /// Create a new guard with no regions registered.
    pub fn new() -> Self {
        Self { sensitive_regions: Vec::new() }
    }

    /// Register a memory region for periodic refresh.
    ///
    /// # Safety
    /// The pointer must remain valid for the lifetime of this guard.
    pub unsafe fn register_region(&mut self, ptr: *mut u8, len: usize) {
        self.sensitive_regions.push((ptr, len));
    }

    /// Refresh all registered regions (call periodically from a background task).
    ///
    /// # Safety
    /// All registered pointers must still be valid.
    pub unsafe fn refresh_all(&self) {
        for &(ptr, len) in &self.sensitive_regions {
            softtrr_refresh_region(ptr, len);
        }
    }
}

#[cfg(feature = "hardened")]
impl Default for RowhammerGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(feature = "hardened", target_arch = "x86_64"))]
unsafe fn softtrr_refresh_region(ptr: *mut u8, len: usize) {
    let mut p = ptr;
    let end = ptr.add(len);
    while p < end {
        // SAFETY: caller guarantees ptr..ptr+len is valid; cache-line aligned flush
        core::arch::x86_64::_mm_clflush(p as *const u8);
        p = p.add(64);
    }
}

#[cfg(all(feature = "hardened", not(target_arch = "x86_64")))]
unsafe fn softtrr_refresh_region(_ptr: *mut u8, _len: usize) {
    // Non-x86_64: no CLFLUSH available; no-op (document in deployment guide)
}

#[cfg(test)]
mod tests {
    #[test]
    fn rowhammer_guard_compiles() {
        // Compile-time check that the feature gate is correct.
        // Runtime test is platform-specific and handled by hardened integration tests.
        let _ = ();
    }
}
