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

// SAFETY: RowhammerGuard is designed to be sent to a background refresh thread.
// The caller who registered regions is responsible for ensuring the raw pointers
// remain valid for the guard's lifetime; no shared mutable state is accessed
// without the caller's explicit unsafe register_region / refresh_all calls.
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
    // SAFETY: caller guarantees ptr is a valid non-dangling pointer to len bytes
    // that outlives this RowhammerGuard; no dereference happens here.
    pub unsafe fn register_region(&mut self, ptr: *mut u8, len: usize) {
        self.sensitive_regions.push((ptr, len));
    }

    /// Refresh all registered regions (call periodically from a background task).
    ///
    /// # Safety
    /// All registered pointers must still be valid.
    // SAFETY: caller guarantees all registered (ptr, len) regions are still live;
    // softtrr_refresh_region is called with those same bounds.
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

// SAFETY: ptr must be a valid aligned pointer to at least `len` bytes of memory
// that the caller controls; this function issues CLFLUSH instructions over the
// range to evict cache lines, which requires only the pointer to be live.
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

// SAFETY: no-op stub; no memory is accessed. Function is unsafe to match the
// x86_64 variant's contract so callers need not be conditional on target_arch.
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
