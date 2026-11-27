//! Helpers for the `#[ignore]`d load tests (`cargo test -- --ignored`): the
//! process's private memory, as the title's memory line reports it.

/// `PrivateUsage` of this process in bytes (0 if the call fails).
pub(crate) fn private_bytes() -> u64 {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    // SAFETY: the counters are plain integers and `cb` is their size.
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        counters.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        let ok = K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            std::ptr::from_mut(&mut counters).cast(),
            counters.cb,
        );
        if ok == 0 {
            0
        } else {
            counters.PrivateUsage as u64
        }
    }
}

/// A cheap mixing step, so noise in "padding" words is spread over all 32 bits.
pub(crate) fn mix(n: u64) -> u32 {
    let mut x = n.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (x ^ (x >> 31)) as u32
}
