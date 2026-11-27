//! Optional comparison of the color views actually passed to OMSetRenderTargets
//! with the formats declared by the graphics PSO of a recorded draw.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};
use windows::Win32::Graphics::Direct3D12::{
    D3D12_RENDER_TARGET_VIEW_DESC, D3D12_RT_FORMAT_ARRAY, ID3D12Resource,
};
use windows::Win32::Graphics::Dxgi::Common::*;

pub(crate) fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| crate::switch::on("VICTORY_RTV_FORMAT_LOG"))
}

/// An unspecified/typeless format cannot establish that two typed views differ.
fn known(format: DXGI_FORMAT) -> i32 {
    match format {
        DXGI_FORMAT_UNKNOWN
        | DXGI_FORMAT_R32G32B32A32_TYPELESS
        | DXGI_FORMAT_R32G32B32_TYPELESS
        | DXGI_FORMAT_R16G16B16A16_TYPELESS
        | DXGI_FORMAT_R32G32_TYPELESS
        | DXGI_FORMAT_R32G8X24_TYPELESS
        | DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS
        | DXGI_FORMAT_X32_TYPELESS_G8X24_UINT
        | DXGI_FORMAT_R10G10B10A2_TYPELESS
        | DXGI_FORMAT_R8G8B8A8_TYPELESS
        | DXGI_FORMAT_R16G16_TYPELESS
        | DXGI_FORMAT_R32_TYPELESS
        | DXGI_FORMAT_R24G8_TYPELESS
        | DXGI_FORMAT_R24_UNORM_X8_TYPELESS
        | DXGI_FORMAT_X24_TYPELESS_G8_UINT
        | DXGI_FORMAT_R8G8_TYPELESS
        | DXGI_FORMAT_R16_TYPELESS
        | DXGI_FORMAT_R8_TYPELESS
        | DXGI_FORMAT_B8G8R8A8_TYPELESS
        | DXGI_FORMAT_B8G8R8X8_TYPELESS
        | DXGI_FORMAT_BC1_TYPELESS
        | DXGI_FORMAT_BC2_TYPELESS
        | DXGI_FORMAT_BC3_TYPELESS
        | DXGI_FORMAT_BC4_TYPELESS
        | DXGI_FORMAT_BC5_TYPELESS
        | DXGI_FORMAT_BC6H_TYPELESS
        | DXGI_FORMAT_BC7_TYPELESS => 0,
        format => format.0,
    }
}

pub(super) fn bound_formats(
    views: &[(
        Option<&ID3D12Resource>,
        Option<D3D12_RENDER_TARGET_VIEW_DESC>,
    )],
) -> [(i32, bool); 8] {
    let mut bound = [(0, false); 8];
    for (slot, (texture, desc)) in bound.iter_mut().zip(views) {
        let format = desc
            .as_ref()
            .map(|d| d.Format)
            .filter(|&f| f != DXGI_FORMAT_UNKNOWN)
            .or_else(|| {
                // SAFETY: the resource is the live host texture of this view.
                texture.map(|t| unsafe { t.GetDesc() }.Format)
            });
        *slot = (format.map_or(0, known), texture.is_none());
    }
    bound
}

fn typed_slots<'a>(
    declared: &'a D3D12_RT_FORMAT_ARRAY,
    bound: &'a [(i32, bool); 8],
) -> impl Iterator<Item = (usize, i32, i32, bool)> + 'a {
    declared
        .RTFormats
        .iter()
        .zip(bound)
        .take(declared.NumRenderTargets as usize)
        .enumerate()
        .filter_map(|(slot, (&format, &(actual, null_view)))| {
            let expected = known(format);
            (expected != 0 && actual != 0).then_some((slot, expected, actual, null_view))
        })
}

pub(super) fn check(
    key: &str,
    declared: &D3D12_RT_FORMAT_ARRAY,
    bound: &[(i32, bool); 8],
    draw_call: &'static str,
    guest_shaders: &str,
) {
    static DRAWS: AtomicU64 = AtomicU64::new(0);
    static COMPARED: AtomicU64 = AtomicU64::new(0);
    static MISMATCHES: AtomicU64 = AtomicU64::new(0);
    const LIMIT: usize = 32;
    type Example = (usize, i32, i32, bool, &'static str);
    static SEEN: Mutex<Vec<(String, Example)>> = Mutex::new(Vec::new());
    static FULL: AtomicBool = AtomicBool::new(false);

    let (mut compared, mut count) = (0, 0);
    for (slot, expected, actual, null_view) in typed_slots(declared, bound) {
        compared += 1;
        if actual == expected {
            continue;
        }
        count += 1;
        if !FULL.load(Relaxed) {
            let example = (slot, expected, actual, null_view, draw_call);
            let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
            if seen.len() < LIMIT
                && !seen.iter().any(|(k, prior)| k == key && *prior == example)
            {
                seen.push((key.to_owned(), example));
                FULL.store(seen.len() == LIMIT, Relaxed);
                drop(seen);
                tracing::info!(
                    pso_key = key,
                    slot,
                    expected_dxgi = expected,
                    bound_dxgi = actual,
                    null_view,
                    draw_call,
                    guest_shaders,
                    "RTV format mismatch"
                );
            }
        }
    }
    COMPARED.fetch_add(compared, Relaxed);
    if count != 0 {
        MISMATCHES.fetch_add(count, Relaxed);
    }
    let draws = DRAWS.fetch_add(1, Relaxed) + 1;
    if draws == 1 || draws >= 65536 && draws.is_power_of_two() {
        tracing::info!(
            draws,
            compared_slots = COMPARED.load(Relaxed),
            mismatched_slots = MISMATCHES.load(Relaxed),
            "RTV format check (VICTORY_RTV_FORMAT_LOG)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_views_keep_their_binding_positions_and_typeless_formats_stay_unknown() {
        let desc = |format| D3D12_RENDER_TARGET_VIEW_DESC {
            Format: format,
            ..Default::default()
        };
        let bound = bound_formats(&[
            (None, Some(desc(DXGI_FORMAT_R10G10B10A2_TYPELESS))),
            (None, Some(desc(DXGI_FORMAT_R11G11B10_FLOAT))),
        ]);
        assert_eq!(bound[0], (0, true));
        assert_eq!(bound[1], (DXGI_FORMAT_R11G11B10_FLOAT.0, true));
        assert_eq!(bound[2], (0, false));
        let mut declared = D3D12_RT_FORMAT_ARRAY {
            NumRenderTargets: 2,
            ..Default::default()
        };
        declared.RTFormats[..2].fill(DXGI_FORMAT_R10G10B10A2_UNORM);
        assert_eq!(
            typed_slots(&declared, &bound).collect::<Vec<_>>(),
            [(
                1,
                DXGI_FORMAT_R10G10B10A2_UNORM.0,
                DXGI_FORMAT_R11G11B10_FLOAT.0,
                true
            )],
        );
        declared.NumRenderTargets = 1;
        assert!(typed_slots(&declared, &bound).next().is_none());
        declared.NumRenderTargets = 2;
        declared.RTFormats[1] = DXGI_FORMAT_R11G11B10_FLOAT;
        let (_, expected, actual, _) = typed_slots(&declared, &bound).next().unwrap();
        assert_eq!(expected, actual);
    }
}
