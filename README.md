# Poor VI

Xbox Series compatibility on Windows, tuned for AMD RDNA2 (RX 6600 XT 8 GB) & 16 GB RAM.

Community compatibility fork built on VIctory and [TaynanGT/Vice-Tay](https://github.com) to run Xbox Series titles on mid-range PC hardware (Ryzen 7 5700G + RX 6600 XT 8 GB + 16 GB RAM).

---

## Status & Benchmarks

| Stage | Status | Notes |
| :--- | :---: | :--- |
| Boot & Startup | ✅ Working | Clean RPF archive loading & D3D12 initialization |
| Menus & Calibration | ✅ Working | Smooth 30.0 FPS (33.3 ms/frame) |
| Loading Screen | ✅ Working | Passes world streaming without AMD RDNA2 crash (0x887A0007 fixed) |
| Intro Cutscenes | ✅ Stable | Runs without GPU device hang (VICTORY_RELAY=0), peaks up to 8–10 FPS |
| In-Game / Gameplay | 🚧 Playable (WIP) | Enters gameplay with peaks up to ~10 FPS on 16 GB RAM (memory optimization ongoing) |

---

## Hardware & Key Fixes

- Test System: AMD Ryzen 7 5700G (8 cores) · Radeon RX 6600 XT 8 GB (RDNA2) · 16 GiB DDR4 + SSD Pagefile · Windows 10/11 x64
- RDNA2 Depth-Stencil UAV Guard (descriptor/host.rs): Prevents invalid UAV creation on depth-stencil textures (0x887A0007 at world load).
- Cutscene GPU Stability (VICTORY_RELAY=0): Prevents AMD driver Device Hung during multi-list cutscene submissions.
- Low-Spec Memory Profile: Textures capped at 512x512 (masks 256x256), 2048 MB local card buffers, 960x540 internal render upscaled to 1080p.

---

## Quick Start

1. Build in release mode:
   ```powershell
   cargo build --locked --release --features diagnostics -p victory
   ```
2. Edit the executable path inside Lancer_Low_GT_VI.bat and double-click it to launch.

---

## Credits & License

- Built upon Marvisak/VIctory, [TaynanGT/Vice-Tay](https://github.com), and upstream RDNA2/AMD community fixes (avatarmanz, codec-d, Belobug).
- Licensed under GPL-3.0-or-later.
