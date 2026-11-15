# Starling development log

This file is the chronological record for future AI assistants and contributors. Keep it updated whenever repository code, configuration, workflows, or project documentation is meaningfully changed.

## 2026-11-15 — Explored map chunk retention

- Repository: `DarkDevil-937/starling-rust-26.3`
- Branch: `starling`
- Code commit: `244385c335043745aa5efdc05d8662ca665e8226`
- Commit message: `Keep explored map chunks after unload`
- File: `crates/horizon-client-render/src/minimap.rs`
- Change: removed the pruning that discarded cached map tiles when chunks unloaded or moved outside the current scan radius. Previously scanned tiles now remain in the in-memory map cache for the current map/dimension session.
- Important limitation: this is RAM-only. It does not yet save explored terrain across app restarts. Cache reset behaviour when switching between different worlds/servers in the same dimension still needs review.
- Build: ARM64-v8a-only workflow run #27, ID `43723719706`, URL https://github.com/DarkDevil-937/starling-rust-26.3/actions/runs/43723719706. Final result checked on 2026-11-15: **success**. ARM64-v8a APK artifact `starling-apk-arm64-v8a` (artifact ID `14165247537`) and symbols artifact `starling-symbols-arm64-v8a` (ID `14166051091`) were uploaded. Workflow run #27 built source commit `244385c...`.
- Device test: not yet confirmed by user.

## Next planned feature — Map image quality / resolution

User reports that fullscreen map terrain looks very low quality / blurry. This is the next priority, ahead of adding more decorative features.

Current implementation observations:
- `crates/horizon-client-render/src/minimap.rs` has `SIZE = 512`.
- Each chunk tile stores 16×16 sampled columns (256 cells).
- `compose()` produces a 512×512 RGBA image from cached chunk tiles.
- `crates/horizon-client/src/extras_hud.rs` draws the map image to the fullscreen HUD viewport.
- A 512×512 composed image stretched over a large phone viewport may look soft; investigate the full texture upload/render path and sampler filtering before blindly increasing SIZE.
- Next engineer/AI should inspect texture creation/upload and sampling (nearest vs linear), viewport scaling/cropping, map zoom-to-blocks-per-pixel, and whether the source image is actually being uploaded at full resolution.
- Target: sharp readable terrain at multiple zoom levels, with smooth pan/zoom and no player movement from map gestures.
- Device: POCO F7; build only `aarch64-linux-android` / ARM64-v8a. Watch memory and frame time; do not trade gameplay performance for a huge texture without measurement.
- No code for this quality improvement has been committed yet.

## 2026-11-15 — Map image quality investigation (diagnostic only)

- No map-quality source code has been changed or committed yet.
- Confirmed `Minimap::SIZE` is 512 and `compose()` creates a 512×512 RGBA image.
- Confirmed the runtime map patch writes the full `SIZE × SIZE` image into the `horizon/minimap` HUD atlas slot. The atlas packer explicitly excludes `horizon/` sprites from shrinking, so this slot should retain its declared resolution; do not assume atlas downscaling is the cause.
- Confirmed the standard HUD atlas sampler is used by the normal map quad. Inspect the sampler defaults and render scaling before changing filtering.
- The map is a block-colour raster: one sampled map colour per world block. Raising texture size alone may simply show more world at the same blocks-per-pixel rather than add detail to each block. The more useful quality control may be fullscreen zoom / blocks-per-pixel and a clear user-facing zoom range.
- Next: inspect fullscreen zoom state and button step logic; verify map texture UVs and sampler behaviour; then make a measured change that increases visible terrain detail without changing player movement or causing avoidable GPU/CPU cost.
- Device validation of image quality: pending; user has not yet tested a new quality build.
## 2026-11-15 — Fullscreen map sampling detail

- Source file: `crates/horizon-client/src/game/extras.rs`.
- Commits: `13cacf9da4c46edf7980ba5214cee7e8b1c35c6d` (`Improve fullscreen map detail by default`) and `b121e010167582aee202c7077a848d2484058c79` (`Use consistent fullscreen map zoom default`). The second commit ensures every fullscreen-map fallback uses the same constant.
- Added `DEFAULT_FULLSCREEN_MAP_ZOOM = 1.0` and used it when no zoom has been selected. Previously the fullscreen map defaulted to 2.0 blocks per pixel, sampling every other world block and losing terrain detail. The new default samples one world block per map texel.
- This does not increase texture size or GPU memory. It shows a closer, more detailed area by default; the existing zoom buttons and pan calculations use the same active zoom value.
- CI: a new ARM64-v8a workflow is expected from the latest push; record its exact run ID/status after checking. No build success claimed yet.
- Device test: pending. User must compare map detail and verify zoom/pan do not move the player.
## 2026-11-15 — Fullscreen map pan marker, session cache, and HUD redesign

- Source commits:
  - `7d9d8fbf4998815ac59b7e3f90c07ab03e5a8475` — `Keep explored map cache and track player while panning` (`crates/horizon-client/src/game/extras.rs`).
  - `9a2e6e31472af3bcab319ead9c65433dc67d8a9e` — `Redesign fullscreen map HUD and move player marker` (`crates/horizon-client/src/extras_hud.rs`).
  - `cedc058e5a97c584b41f89de9fc651c2e4d84347` — `Set sharper closer fullscreen map starting zoom` (`crates/horizon-client/src/game/extras.rs`).
- Cache lifecycle: stopped clearing the explored tile cache merely because `player` becomes `None` on the title/disconnect screen. The in-memory explored tiles can now survive leaving a world/server and returning while the app process remains alive. Cache still clears on dimension changes and when the map is explicitly disabled. This is not disk persistence; force-stopping/restarting Starling loses this cache. A world/server identity key is still needed to prevent old cached terrain mixing with a different world in the same dimension.
- Player marker: fullscreen map now computes a screen-space offset from `map_pan` and zoom, so the marker represents the player's actual position relative to the panned terrain instead of staying pinned at map centre. It is hidden when the player is outside the visible map.
- Fullscreen HUD: replaced the raised pixel/bevel buttons with restrained flat translucent controls and a compact WORLD MAP / coordinate strip. Existing button positions and touch hitboxes are unchanged.
- Starting zoom: set `DEFAULT_FULLSCREEN_MAP_ZOOM` to `0.5` blocks per pixel so terrain appears closer/larger on screen. This does not create sub-block data; it trades visible world area for a larger presentation of the same block-level map pixels. User can zoom out with the existing control.
- Build: ARM64-v8a-only workflow triggered by latest pushes; record run ID, status and artifacts after checking. No build success claimed yet.
- Device validation: pending. Specifically test (1) drag map and confirm player marker moves relative to terrain, (2) leave/rejoin same world/server and confirm explored map remains, (3) open fullscreen map and judge visual layout and detail, (4) switch to a different world/dimension and report any stale terrain.
## Required validation checklist

For every code change:
1. Record date, branch, commit SHA/message, exact files changed, and what changed.
2. Record any known limitations or risks; distinguish implemented from planned.
3. Record the exact GitHub Actions run ID/URL and ABI; record status and conclusion only after checking.
4. Record whether the user tested it on device and the actual result. Never imply device testing happened if it did not.
5. Keep `PROJECT_HANDOFF.md` in sync with the latest state and point to this log.
6. Preserve ARM64-v8a-only builds unless the user explicitly changes that requirement.


## 2026-11-15 — Persistent explored map, waypoint controls, and fullscreen remaster

- Branch: `starling`.
- Source commits:
  - `47ada6624c4c133580462cd46c2e0725224ab170` — add serializable explored-tile cache load/save in `crates/horizon-client-render/src/minimap.rs`.
  - `a0295c1e8eab1662b19d12895924509675556d56` — integrate cache load/save into `crates/horizon-client/src/game/extras.rs`.
  - `96a34af0c6e126a53c92c0a1d0bbea8de1e93a2b` — add saved waypoint state and touch controls.
  - `27cbb731cfb492cebf73ef0c835b9c5c74554e47` — redesign fullscreen map header/footer and render waypoint markers.
  - `b8ce99588a61764830f2184e49696844f63dbd32` — raise map raster from 512 to 1024 texels.
  - `b9e009d7cff91710661b0d97635269bb5e409875` — remove unused imports.
  - `4a003bba5858e6f6ed0c51eb1a4c0c8bd87b983d` — include server identity in cache key and add remove-last-waypoint action.
  - `224aa27e155106a65e69eb3607e39dc5185049a2` — add the matching delete waypoint control to the HUD.
  - `ab69ee7552034fdaa8cde18ca281b1b5e588b9f9` — use 0.25 blocks per texel for the 1024px raster and allow finer zoom.
  - `a6bb56350d318827959da70dd5f8f20fcd8dba4e` — update map raster documentation.
- Explored-chunk persistence: tiles are serialized as versioned JSON under `HORIZON_PRIVATE/map-cache/`, with a key derived from the world root, connected server address (or singleplayer), and dimension. The cache is loaded on entering a world, saved periodically while exploring (3-second cadence when dirty), and flushed when leaving to the title/disconnect screen. Writes use a temporary file followed by rename so an interrupted write does not replace the last complete cache.
- Waypoints: W button adds an automatically named waypoint at the player's current X/Z; D removes the most recently added waypoint. Waypoint coordinates/names are saved separately per world/dimension and are drawn on the fullscreen map at their world-relative positions.
- Fullscreen UI: full-width title/coordinate header, compact bottom legend, waypoint markers, and close/zoom/add/remove controls. Map gestures remain captured while the map is open.
- Quality: map image is now 1024×1024 (4× the previous pixel count), with default zoom 0.25 blocks per texel. This gives a denser fullscreen raster and a finer zoom range. The underlying terrain source is still one sampled map colour per world block; this is not sub-block geometry or a photorealistic terrain renderer.
- Performance risk: composing a 1024×1024 image processes four times as many pixels as 512×512, and each upload carries a 4 MiB RGBA image. Validate frame pacing and memory on the POCO F7.
- Build/CI: not yet verified for these commits. Do not install an earlier artifact as this update.
- Device test: pending. Verify restart persistence, separation between singleplayer/server/dimensions, waypoint add/remove/persistence, map quality, pan/zoom, and gameplay input isolation.
- Remaining waypoint limitation: current first pass auto-names markers and removes only the last marker; rename/select/delete-individual waypoint management is not implemented yet.

- Follow-up source fixes after the first log entry: `641bd654021fdda9dd13c50abf5fb25aff4ef5d2` uses an explicit string slice for the cache-key comparison and avoids moving the waypoint path during the atomic rename; `f30aa07639d7f445780db5ef3a561d5cacaa5e95` adds a disk-cache round-trip unit test; `b105d832c65282838ba595e6f0f7452e5261667b` makes its temporary test filename portable. These are part of the same unverified map-remaster work.

- Waypoint interaction follow-up: `edd7986391bdf8dfe6d9efefdfadbbd0d23562cf`, `17cc3e7407dd2235c9ca9aed17c03af12153e48b`, and `2e87e1ec1ecc4b9d65421ff6e72dfa89f40b753d` add tap-to-select waypoint markers, highlight the selected marker, and make D delete the selected marker (or the last marker if none is selected). Waypoints remain automatically named in this first pass.

- Final source-format follow-ups: `116c662fb9dabd0d6b7e14a2a26e36024787078b`, `203aaa70efeb6f845333b092dad847407fb163fb`, `7cf868d9c255ad24d836bfa47cf5d13f2b0cabce`, `83837928fb9bbac20a8224b91da9171b39a06599`, and `bdd807d3ada5acc91316937855589956763b74f7` format the map interaction/cache code and order imports. These do not change the intended feature behavior.

- Zoom range follow-up: `51bbb7962e36a98c76c4e568431a960a8018e2da` keeps zoom-out's lower clamp aligned with the finer 0.125 blocks-per-texel minimum, avoiding a jump when zooming back out from the closest setting.

- Rustfmt follow-up: `fc2f64a0607276413c2af18f415bee7421a856b7` expands the cache timestamp helper to standard Rust formatting. This is formatting-only.

 
## 2026-11-15 — Remove upstream-only GitHub Actions workflows

- Removed seven workflows that are explicitly scoped to the upstream `horizon-rust/horizon` repository or its `main` branch and do not provide Starling ARM64 APK builds or checks on this fork's `starling` branch:
  - `.github/workflows/claim-command.yml`
  - `.github/workflows/mod-registry.yml`
  - `.github/workflows/mojang-republish.yml`
  - `.github/workflows/native-hook-stamp.yml`
  - `.github/workflows/overlap-note.yml`
  - `.github/workflows/parity-main.yml`
  - `.github/workflows/pr-labels.yml`
- Kept `android.yml` (ARM64 APK build), `ci.yml` (Rust checks), `cancel-on-close.yml` (cancel abandoned PR runs), `native-mods.yml` (native mod builds), and `pack-bench.yml` (manual/resource-pack benchmark).
- No workflow artifacts or build history were deleted; this change only removes unused workflow definitions.
- A push to `starling` will trigger the retained Android and checks workflows. Confirm both finish successfully before downloading a new APK.

- CI diagnosis and formatting repair (2026-11-15): checks run #72 (`43733840471`) failed only at `Check formatting and lint`; the captured rustfmt diff identified formatting in `extras_hud.rs`, `game/extras.rs`, and `minimap.rs`. No Rust compilation failure was reported by that job because lint/format stopped the remaining tests. Formatting was applied in commits `471eab6e5099ad954fa6db4533e94ca1ab09c911`, `c9d7444b932f063254472f6af5b2479e8bf9c8d7`, and `e4dfe0f420503241450053fc25d2d38fff9efc2e`. Rechecks #74/#75 are running; the earlier ARM64 build #76 (`43733840669`) is still running on its earlier SHA. Later Android workflow runs were cancelled by newer pushes/concurrency; this is cancellation, not a compile failure.


## Map remaster build repair — 2026-11-15

Actions #78 and #82 exposed stale workflow-presence tests after removing upstream-only workflows, plus minimap cache compilation errors. Removed only workflow-presence assertions while retaining functional tests; aligned the runner test with the consolidated single-job checks workflow. Minimap tile arrays now use serde-compatible vectors with 256-entry validation on load; serialized keys are copied to avoid an elided-reference lifetime error. Removed upstream pack-bench and PR-close cleanup workflows; retained Android ARM64 APK, Rust checks, and native-mod builds. Await fresh CI and ARM64 validation.

## 2026-11-15 — Fullscreen map cursor and waypoint placement refinement

- User reports explored chunks persist after restart and map quality is now acceptable; fullscreen UI and waypoint placement still need improvement.
- Source changes in this update:
  - crates/horizon-client/src/game/extras.rs: added a virtual map cursor; tapping the map positions it independently of the player. Add waypoint converts cursor screen coordinates back into world X/Z using the map zoom/viewport transform. Arrow keys move the cursor and Enter activates the control under it.
  - crates/horizon-client/src/extras_hud.rs: replaced the full-width header with a compact left-side WORLD/coordinates/waypoint panel; added a visible crosshair cursor and cursor world-coordinate readout; retained full-screen terrain and right-side map buttons.
- Build status before this update: Android run #83 (ID 43735462980) succeeded for ARM64-v8a and produced artifact starling-apk-arm64-v8a (artifact 14170610432). Checks run #79 (ID 43735462920) failed only at cargo fmt --check due to formatting in game/extras.rs and minimap.rs; no compilation/tests were reached.
- The available GitHub connection exposes Actions run/artifact reads and job reruns, but no workflow-run deletion operation. Historical run deletion was not performed.
- Pending: run rustfmt, inspect cursor/world-coordinate transform, run checks and ARM64 APK workflow, then device-test cursor placement, button activation, and player-input isolation.


## 2026-11-15 — CI formatting repair

- Inspected checks run #83 on branch `starling` (commit `8b5246749165bb4ab920a8b65070ceb590fcf01c`). The failure was specifically `cargo +stable fmt --all -- --check`, with formatting diffs only in `extras_hud.rs` and `game/extras.rs`; clippy had not run yet.
- Applied the exact rustfmt layout shown in the CI log to those two files. Android run #87 on the same source tree was still building ARM64-v8a at inspection time; its result is not yet treated as validation.
- Process improvement: inspect failed job logs and repair the reported cause before asking the user to rerun anything. Only recommend downloading the APK artifact from the final successful ARM64 Android run whose commit contains the cursor/waypoint changes.


## 2026-11-15 — Second CI formatting correction and build result

- Checks run #84 (ID 43738219505) on commit `8bbd72d00879caedbe98aadefa1b79c1382feccb` failed at `cargo +stable fmt --all -- --check` due to one remaining line-wrap difference in `crates/horizon-client/src/game/extras.rs`. Clippy and Rust tests were not reached.
- Android run #88 (ID 43738219528) for the same commit succeeded. It produced `starling-apk-arm64-v8a` (artifact ID 14171324177) and `starling-symbols-arm64-v8a` (artifact ID 14171319351). The APK is built, but full CI validation remains incomplete until checks pass.
- Applied the exact remaining rustfmt line wrapping to `game/extras.rs`. A new push should trigger fresh checks and ARM64 build; prefer the artifact from that final commit once both workflows pass.


## 2026-11-15 — CI lint diagnosis and fix

- Checks run #85 (ID 43739638523) for commit `842f7c06bf747ff1c9a9ed75b8bc4bc8a801e515` passed rustfmt and repository Python tests, then failed during Rust clippy/compile with `-D dead-code`: `Game::fullscreen_map_open` is referenced by the Android touch HUD but unused in the host CI feature/target configuration.
- Kept the method for the Android call site and annotated it with a narrowly scoped `#[allow(dead_code)]`, with a comment explaining the target-specific use. No blanket lint suppression.
- Android run #89 (ID 43739638532) succeeded for ARM64-v8a and built an APK from the same commit, but do not recommend it yet: the checks failed. Await a fresh checks run and a matching successful ARM64 run for this lint fix.


## 2026-11-15 — Remove unused fullscreen-map accessor after Checks #85

- Android run #89 (ID 43739638532) succeeded on commit `842f7c06bf747ff1c9a9ed75b8bc4bc8a801e515`; it produced `starling-apk-arm64-v8a` artifact ID `14171568404` (34,456,064 bytes) and symbols artifact ID `14171688053`.
- Checks run #85 (ID 43739638523) passed formatting and Python checks, then failed Rust lint/build because `fullscreen_map_open` in `crates/horizon-client/src/game/extras.rs` was unused under warnings-as-errors. Rust core tests were not reached.
- Removed the unreferenced accessor rather than suppressing the dead-code lint. The map-open state remains used by the existing map input/touch flow; this deletion only removes the unused getter.
- Await new checks and ARM64 workflow results for this cleanup commit. The previous APK was built but is not the recommended install until checks are green.
