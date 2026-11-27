# Starts the title with the settings this fork is played with (README.md, "Getting started").
#   scripts\play.ps1 -Exe <path of your own copy's executable>
# -Safe         without the newer, faster ways (the first thing to try when something is wrong)
# -RayTracing   the title's own ray-traced indirect light: lit interiors, about a third slower
# -Windowed     a window instead of the whole screen (full screen is the default)
# -NoPanel      without the performance panel (on by default, F3 hides it while playing)
# -Console      series-s (the default) or series-x: the console the title takes this PC for
# -Build        target\diag (the default, the build all measurements are of) or target\release
# -Machine      auto (the default: by the graphics card), 6600xt, 7900xtx, 4070 or 4090
#               6600xt: Ryzen 7 5700G (8 cores), Radeon RX 6600 XT 8 GB, 16 GB RAM; the low-memory
#               Series S profile (title told 5.75 GB, textures kept at 512 / 256, mip bias +1.5,
#               light estimate every 4th frame, the title's own upscaler at half scale)
# -More         other settings: -More @{VICTORY_PANEL='1'} or -More @{VICTORY_AMBIENT='1.0'}
# On a smaller NVIDIA PC, compare the same Series S scene after a fresh start
# with one override at a time, e.g. -More @{VICTORY_LAND_THREADS='3'} for
# copy-back work, or -More @{VICTORY_THIN_THREADS='6'} for list translation.
# Run it from a normal window, not as administrator: the driver then keeps its shader cache elsewhere.
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$Safe,
    [switch]$RayTracing,
    [switch]$Windowed,
    [switch]$NoPanel,
    [ValidateSet('series-x', 'series-s')][string]$Console = 'series-s',
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [ValidateSet('auto', '6600xt', '7900xtx', '4070', '4090')][string]$Machine = 'auto',
    [hashtable]$More = @{}
)
$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_TITLE_PRIORITIES = '0'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_D3D12_DRED = 'light'
}
if ($Machine -eq 'auto') {
    # Use the existing vendor defaults. The 4070 / 7800X3D profile is explicit.
    $cards = @(Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | ForEach-Object { $_.PNPDeviceID })
    $names = @(Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | ForEach-Object { $_.Name })
    $Machine = if ($names -match 'RX 6[456][0-9]0') { '6600xt' } elseif ($cards -match 'VEN_10DE') { '4090' } else { '7900xtx' }
}
if ($Machine -eq '7900xtx' -or $Machine -eq '6600xt') {
    # (Also the RX 6600 XT, before its own settings below.)
    # i9-12900K, RX 7900 XTX, 64 GB: the page-mapping lock was fastest with one
    # thread, and the AMD driver lowers its clocks between submissions.
    $settings.VICTORY_MAP_THREADS = '1'; $settings.VICTORY_CARD_AWAKE = '1'
} else {
    # RTX 4090, Ryzen 9 9950X3D (16 cores of one kind, two caches), 48 GB.
    # NVIDIA sets its clocks by the driver's power mode: no spinner on the
    # compute queue. 16 cores record and land, not 8.
    $settings.VICTORY_THIN_THREADS = '12'; $settings.VICTORY_UNDER_THREADS = '8'; $settings.VICTORY_LAND_THREADS = '6'
    if (-not $Safe) {
        # Pages are mapped by several threads only while the title streams
        # (not the default: one run behind it).
        $settings.VICTORY_MAP_THREADS = 'auto'
        # A fill of the constant-buffer ring that a small copy at its place writes
        # into refused the whole submission to the card steps; in list order it is
        # taken (submit thread 104.7 -> 50.9 ms a frame, 12.1 -> 14.3 fps here).
        $settings.VICTORY_PLACED_ORDER = '1'
    }
    # The console wave64 programs a 32-lane card takes only in the wave-wide
    # form; this list of them ran without a lost device on this card for 30
    # minutes (the losses of runs 121-124 had the form on for every program).
    # Kept under -Safe too: without it those programs do not translate and
    # their draws are dropped, the picture -Safe is there to check.
    $settings.VICTORY_WAVE_WIDE_ONLY = '6f225b641c5b0708,5dc3b25986622949,369cbe906ea72764,2cc4c197ee29e291'
}
if ($Machine -eq '4070') {
    # Ryzen 7 7800X3D: eight physical cores, versus the 4090 test PC's sixteen.
    # Keep the same wave64 allowlist and rendering profile; spend VRAM on
    # shader buffers before textures consume it. No approximate shader modes.
    $settings.VICTORY_THIN_THREADS = '8'; $settings.VICTORY_UNDER_THREADS = '6'; $settings.VICTORY_LAND_THREADS = '4'
    $settings.VICTORY_MAP_THREADS = '1'; $settings.VICTORY_PIPELINE_WORKERS = '3'
    if (-not $Safe) {
        $settings.VICTORY_CARD_RESERVE_BUDGETS = '1'
        $settings.VICTORY_CARD_BUFFERS_MIN_KB = '64'; $settings.VICTORY_CARD_BUFFERS_MB = '1536'
        $settings.VICTORY_CARD_WINDOWS_MB = '1024'
    }
}
if ($Machine -eq '6600xt') {
    # Ryzen 7 5700G (8 cores, one complex), RX 6600 XT (RDNA2, 8 GB), 16 GB DDR4:
    # the title must stay under 11.5 GB of RAM and 6 GB of card memory.
    $settings.VICTORY_THIN_THREADS = '8'; $settings.VICTORY_UNDER_THREADS = '6'; $settings.VICTORY_LAND_THREADS = '4'
    $settings.VICTORY_MAP_THREADS = 'auto'
    $settings.VICTORY_CARD_RESERVE_BUDGETS = '0'
    $settings.VICTORY_CARD_BUFFERS_MB = '512'; $settings.VICTORY_CARD_WINDOWS_MB = '512'
    $settings.VICTORY_POOL_SMALL = '0'
    $settings.VICTORY_CARD_AWAKE = '1'
    # Low-memory Series S profile (each is also the build's default on Series S;
    # written out so that -More can change one and the log shows them).
    $settings.VICTORY_LEAN_RAM = '1'
    $settings.VICTORY_TITLE_BUDGET_MB = '5888'     # the title is told 5.75 GB: its own streaming collector runs early
    $settings.VICTORY_TEXTURE_MAX = '1024'          # colour and normal maps: levels above 512 texels are never read
    $settings.VICTORY_TEXTURE_MAX_MASK = '256'     # single-channel masks
    $settings.VICTORY_LOD_BIAS = '1.5'             # mip bias added to filtering samplers
    $settings.VICTORY_AMBIENT_EVERY = '4'          # the light estimate's cells, one frame in four
    $settings.VICTORY_OWNERLESS_VIEWS = '100000'
    $settings.VICTORY_LAND_SKIP = '1'
    # 960x540 inside, 1920x1080 out, by the title's own upscaler (6). Never 7
    # (FidelityFX): it lost the device (0x887A0007) on this card.
    $settings.VICTORY_OUTPUT = '1920x1080'
    $settings.VICTORY_UPSCALER = '6'
    $settings.VICTORY_RENDER_SCALE = '0.5'
    $settings.VICTORY_AMBIENT = '0.5'
    $settings.VICTORY_CARD_AWAKE = '1'
}
# -Safe leaves out every way still on trial, and the newest faster ones.
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    $settings.VICTORY_THIN = '1'; $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
    # A dispatch does not end a batch of argument clamps before the title waits
    # for it: off by default, as other ways the title may wait are not known.
    $settings.VICTORY_INDIRECT_BATCH = '2'
}
if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
# The whole screen, with no frame, as on the console.
if (-not $Windowed) { $settings.VICTORY_FULLSCREEN = '1' }
# The performance panel (FPS, frame times, GPU, CPU, VRAM, RAM) in the upper left
# corner, so clips and screenshots show what the build runs at and on what PC.
if (-not $NoPanel) { $settings.VICTORY_PANEL = '1' }
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }

$ram = (Get-CimInstance Win32_ComputerSystem -ErrorAction SilentlyContinue).TotalPhysicalMemory
Write-Host "Settings for the $Machine$(if ($Safe) { ', safe' })$(if ($RayTracing) { ', with ray tracing' })."
if ($Console -eq 'series-s' -and $Machine -eq '4090') {
    Write-Host ("Series S requests {0} list translation threads, {1} for memory updates, and {2} for landing." -f
        $settings.VICTORY_THIN_THREADS, $settings.VICTORY_UNDER_THREADS, $settings.VICTORY_LAND_THREADS)
}
if ($ram -and $ram -lt 60GB) {
    Write-Host ("Windows reports {0:N1} GiB of physical RAM. Keep a system-managed page file. See docs/measurements.md for measured Series S memory use." -f ($ram / 1GB))
}

$victory = Join-Path $PSScriptRoot "..\target\$Build\victory.exe"
if (-not (Test-Path $victory)) { Write-Host "No $victory. Build it first (README.md)."; exit 1 }
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; exit 1 }
# (Other settings left over in this window go; where your files are stays.)
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR'
$inherited = @(Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths })
foreach ($entry in $inherited) { Remove-Item "Env:$($entry.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
$discarded = $inherited | Where-Object {
    $_.Name -notin $More.Keys -and -not $_.Value.Equals([Environment]::GetEnvironmentVariable($_.Name))
} | Sort-Object Name
if ($discarded) {
    Write-Host "Discarded inherited values for $($discarded.Name -join ', '). Use -More to pass them explicitly."
}
$Exe = (Resolve-Path $Exe).Path
Push-Location (Join-Path $PSScriptRoot '..')   # victory.toml is looked for here
try { & $victory run $Exe --console $Console --execute } finally { Pop-Location }
