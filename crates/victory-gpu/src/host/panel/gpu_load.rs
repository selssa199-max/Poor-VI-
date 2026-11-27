//! Windows GPU engine utilization for this process and its rendering adapter.

use std::mem::{MaybeUninit, size_of};
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::time::Duration;

use windows::Win32::Graphics::Direct3D12::ID3D12Device;
use windows::core::PCWSTR;
use windows_sys::Win32::System::Performance::{
    PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};

const UNAVAILABLE: u32 = u32::MAX;
static SAMPLE: AtomicU32 = AtomicU32::new(UNAVAILABLE);

pub(super) fn latest(device: &ID3D12Device) -> Option<u32> {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        // SAFETY: the presenter owns a live D3D12 device.
        let luid = unsafe { device.GetAdapterLuid() };
        let prefix: Vec<u16> = format!(
            "pid_{}_luid_0x{:08x}_0x{:08x}_phys_",
            std::process::id(),
            luid.HighPart as u32,
            luid.LowPart
        )
        .encode_utf16()
        .collect();
        let _ = std::thread::Builder::new()
            .name("victory-panel-gpu".to_owned())
            .spawn(move || poll(&prefix));
    });
    let value = SAMPLE.load(Relaxed);
    (value != UNAVAILABLE).then_some(value)
}

// PDH can query the system-wide engine list. Keep it off the present thread.
fn poll(prefix: &[u16]) {
    let Some(mut meter) = Meter::new() else {
        return;
    };
    let mut primed = false;
    loop {
        let value = if super::shown().load(Relaxed) {
            // SAFETY: the query belongs to this thread.
            let ready = unsafe { PdhCollectQueryData(meter.query) } == 0;
            let value = (primed && ready).then(|| meter.read(prefix)).flatten();
            primed = ready;
            value
        } else {
            primed = false;
            None
        };
        SAMPLE.store(value.unwrap_or(UNAVAILABLE), Relaxed);
        std::thread::sleep(Duration::from_secs(1));
    }
}

struct Meter {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    buffer: Vec<MaybeUninit<PDH_FMT_COUNTERVALUE_ITEM_W>>,
}

impl Meter {
    fn new() -> Option<Self> {
        let mut query = null_mut();
        // SAFETY: null data source selects the live counters; the output is local.
        if unsafe { PdhOpenQueryW(null(), 0, &mut query) } != 0 {
            return None;
        }
        let mut meter = Self {
            query,
            counter: null_mut(),
            buffer: Vec::new(),
        };
        let path: Vec<u16> = r"\GPU Engine(*)\Utilization Percentage"
            .encode_utf16()
            .chain([0])
            .collect();
        // SAFETY: the query is live and the counter path is nul-terminated.
        (unsafe { PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut meter.counter) } == 0)
            .then_some(meter)
    }

    fn read(&mut self, prefix: &[u16]) -> Option<u32> {
        let (mut bytes, mut count) = (0, 0);
        // SAFETY: a null destination asks PDH how much space it needs.
        let probe = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                null_mut(),
            )
        };
        if probe != PDH_MORE_DATA || bytes == 0 {
            return None;
        }
        self.buffer.resize(
            (bytes as usize).div_ceil(size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>()),
            MaybeUninit::uninit(),
        );
        // SAFETY: the buffer is aligned and holds at least the requested byte count.
        let got = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                self.buffer.as_mut_ptr().cast(),
            )
        };
        if got != 0 || count as usize > self.buffer.len() {
            return None;
        }
        // SAFETY: a successful call initialized the reported elements.
        let items = unsafe {
            std::slice::from_raw_parts(
                self.buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                count as usize,
            )
        };
        // As in Task Manager, use the busiest engine. Engines can overlap.
        items
            .iter()
            .filter_map(|item| {
                if !matches!(
                    item.FmtValue.CStatus,
                    PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
                ) || item.szName.is_null()
                {
                    return None;
                }
                // SAFETY: PDH supplies a nul-terminated name and the requested double value.
                let name = PCWSTR(item.szName);
                let name = unsafe { name.as_wide() };
                let value = unsafe { item.FmtValue.Anonymous.doubleValue };
                (starts_with_ignoring_case(name, prefix) && value.is_finite())
                    .then_some(value.clamp(0.0, 100.0).round() as u32)
            })
            .max()
    }
}

/// Windows spells the adapter LUID in the instance name in upper-case hex
/// ("luid_0x00000000_0x0000D1E6"); the prefix is built in lower case.
fn starts_with_ignoring_case(name: &[u16], prefix: &[u16]) -> bool {
    let lower = |c: u16| if (b'A' as u16..=b'Z' as u16).contains(&c) { c + 32 } else { c };
    name.len() >= prefix.len() && name.iter().zip(prefix).all(|(&n, &p)| lower(n) == lower(p))
}

impl Drop for Meter {
    fn drop(&mut self) {
        // SAFETY: the query is live and owns its counter.
        unsafe { PdhCloseQuery(self.query) };
    }
}

#[cfg(test)]
mod tests {
    use super::starts_with_ignoring_case;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn the_adapter_prefix_matches_in_either_case() {
        let name = wide("pid_7_luid_0x00000000_0x0000D1E6_phys_0_eng_0_engtype_3D");
        assert!(starts_with_ignoring_case(&name, &wide("pid_7_luid_0x00000000_0x0000d1e6_phys_")));
        assert!(!starts_with_ignoring_case(&wide("pid_70_luid_0x0"), &wide("pid_7_luid_")));
        assert!(!starts_with_ignoring_case(&wide("pid_7"), &wide("pid_7_luid_")));
    }
}
