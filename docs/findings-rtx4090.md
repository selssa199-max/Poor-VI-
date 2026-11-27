# What was found on the RTX 4090 (2026-11-26)

The test PC the README describes: RTX 4090, Ryzen 9 9950X3D, 43.4 GB of RAM
visible to Windows. Series S profile, diagnostics builds, the same boat scene at
the start of every run. Runs are 30 minutes unless said otherwise.

## Memory that never came back (#27)

The game kept growing: 36 GB after 25 minutes, 52 GB after 70, past physical
RAM, and the frame rate fell from about 20 fps to 8-10.

Views of placed memory (device methods 0x230 and 0x238) were keyed by address
plus a shape number taken from the view and resource descriptions. The title
leaves some words of those descriptions unwritten, so the same view kept
arriving with different garbage and made a new shape each time. Counting
distinct values per word over the first million shapes showed which ones:

| word | distinct values |
|---|---|
| resource description, word 1 (padding after Dimension) | 33,403 |
| resource description, word 13 (padding after Flags) | 12,846 |
| SRV, word 3 (padding after Shader4ComponentMapping) | 131,392 |
| view, word 9 (unused tail of the union) | 36,413 |

After about 30 minutes the shapes filled their 2^22 slots and every later view
became a permanent view-table entry: 34.9 million entries and 16.7 GB after 70
minutes.

The fix zeroes those words, and the union words each SRV/UAV dimension does not
use, before keying. Entries no resource releases are capped at a million
(`VICTORY_OWNERLESS_VIEWS`), and the memory line counts views instead of walking
the table. Memory then stayed at 27 GB for the whole 30 minutes.

## A lock every list thread queued on (#37)

After merging avatarmanz up to c9c932cb, the game dropped to 6 fps about two and
a half minutes into the world, with the CPU and the card mostly idle. The title
waited 120-144 ms a frame at the present for the submission line.

- `VICTORY_TIMELINE` showed list calls of 70-250 ms that used no CPU.
- A PIX timing capture (`pixtool attach PID take-new-timing-capture FILE
  --duration=3000`), read through its context switches and their stacks, put the
  list threads in `Mutex::lock_contended` inside `shadow::exact`, woken by other
  list threads leaving the same function. They spent about 40% of the capture
  there. The capture had no symbols for our image; addresses were named with
  `llvm-symbolizer` after finding the load address from return addresses that
  must follow a call.
- `shadow::exact` makes a buffer of a view's own bytes while holding a global
  lock. gomeaw's b68e65e0 sends there every structured view whose slot has no raw
  twin and that begins off 16 bytes. An RTX 4090 refuses a heap with raw twins
  (it takes 1,000,000 views), so the rule applied to every such view: 265 to
  1,968 requests and 16 to 313 new buffers a frame, against 1.5 and 0.5 before.

The rule now sits behind `VICTORY_STRUCTURED_RAW=1`. With it off the same merge
ran at 22.7 fps.

Ruled out on the way: paging (with 4 KiB textures the game fit in RAM and still
ran at 6.5 fps), `VICTORY_CACHE_CORES`, `VICTORY_AMBIENT_SETTLE`,
`VICTORY_SHOW_NEW_LATER`, `VICTORY_SPANS_QUICK` and `VICTORY_SYNC_IN_PLACE`
(all off at once: 6.3 fps), and the second batch of fixes (the merge without it
was as slow).

## Small textures at 64 KiB (#45)

c278f0d5 placed every pooled texture at 64 KiB multiples after two device losses
on an RX 7900 XTX. Here it only cost memory: 3.2 GB more of the PC's and 3.4 GB
more of the card's, at the same frame rate. Cards other than AMD's are back to
4 KiB; `VICTORY_POOL_SMALL=1` or `=0` picks either.

## Tried and left as it is

- `VICTORY_LATE_WAIT_LISTS=512`: the wait for lists in the air is 0.0 ms a frame
  here, so a higher limit has nothing to win.
- Skipping the four impostor pixel shaders flamingo's cea3ca58 points at
  (`VICTORY_SKIP_PIPELINES=7115e46a79c78f98,59b1ba494f18c894,b8ca32a6899b87fb,659fe9cae59dbd88`):
  22.5 fps against 21.5 in 8 minutes; whether it clears the flat black shapes is
  still to be checked.
