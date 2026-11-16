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


## 2026-11-15 — Android map guard and no-audio host lint repair

- Latest prior commit `0ff99a4ac6eeece14045ae6ac8f8e324222d444c` removed the unused `fullscreen_map_open()` accessor but left one Android-only call in `game/mobile_controls.rs`. The ARM64 build correctly failed with E0599 at that call.
- Replaced that call with the existing state field `self.extras.big_map`, preserving the mobile HUD suppression while the fullscreen map is open.
- Checks #87 also failed in `horizon-client-render/src/audio.rs`: the host checks build disables the `audio` feature, so `Stream = Infallible` and matching `Ok(v)` is unreachable. Scoped stream opening/matching to `#[cfg(feature = "audio")]` and use `(None, false)` for no-audio builds. Audio-enabled Android builds retain the existing open/fallback behaviour.
- Updated source only; this commit is **not yet CI-verified** and no APK is ready until Checks and ARM64-v8a Android build both pass.


## CI repair — 2026-11-15 (runs #92 and #88)

- Inspected failed Android run #92 (job 132896721038) and Checks run #88 (job 132896720784) for commit `fa6c258d70213409a25b81d141334d3b81b1ce87`.
- Android's exact compiler error was E0616: `mobile_controls.rs` reads `ExtrasState.big_map`, but the field was private to the sibling `extras` module. Narrowly changed the field to `pub(super)`, exposing it within the parent `game` module only.
- Checks' exact error was `dead_code` on the no-audio `Audio::open_named` stub. That stub has no callers because `set_device` is separately compiled for no-audio builds. Removed the unused stub; the real `open_named` remains compiled only with `feature = "audio"`.
- These are source-level corrections to the two actual errors; fresh checks and the ARM64-v8a Android build are required before considering the repair validated. No APK from #92 was produced.


## 2026-11-15 — Xaero-inspired fullscreen map HUD pass

- User supplied two screenshots: current Starling fullscreen map and a Xaero-style reference. The requested target is a map-first fullscreen layout with a slim dark left rail, compact coordinate strip near the top centre, compass at the upper right, compact right-side controls, and a zoom readout at the bottom.
- Updated `crates/horizon-client/src/extras_hud.rs` to replace the oversized upper-left WORLD card and large right-side buttons with that hierarchy. Terrain remains full-screen; map pan/zoom, cursor, waypoint markers, and existing control hitbox return values are retained. The close/zoom/add/delete controls remain the same five actions.
- This is a visual HUD pass only. It does not add biome-name detection, settings menu functionality, new map sampling/detail, or change terrain coverage. The left rail's waypoint count is informational.
- CI validation is pending. Previous Android run #93 succeeded for ARM64-v8a on commit `274988e4a3dcb84e2cc58e5d984c0df7b26a0c45`; Checks #89 failed in two minimap tests. Those test expectations must be corrected separately after inspecting the exact assertions/logs. The #93 APK is not this new UI pass.
- Device validation of the Xaero-style layout is pending; do not claim the UI is complete until user tests the new build on POCO F7.


## 2026-11-16 — Xaero-style HUD formatting CI fix

- Checked Android workflow #94 and Checks workflow #90 for commit `f09f8ed55c1c2eabe616210e6f93eddd838d1d2f`.
- Android #94 completed successfully and published the verified `starling-apk-arm64-v8a` artifact (34,459,570 bytes).
- Checks #90 failed only at `cargo +stable fmt --all -- --check`. The reported diffs were formatting in `crates/horizon-client/src/extras_hud.rs` (compass labels and two long `batch.fill` calls); the workflow did not reach Clippy or Rust tests.
- Applied the exact rustfmt layout requested by the CI diff. No runtime behaviour was intentionally changed.
- This formatting correction is committed separately and triggers fresh Checks and ARM64-v8a Android workflows. Await those results before treating the corrected branch head as fully validated.


## 2026-11-16 — Xaero-like map screen: no button panel, built-in pointer, tiny zoom buttons

User feedback that drove this pass: the fullscreen map still had a row of on-map buttons, the gameplay controls were visible over it, and the "cursor" should not be a new drawing at all — the game already ships a virtual mouse pointer (the Android touch HUD's pointer) and that is the one to show. The user asked for Xaero's layout instead: clean map, small zoom in/out buttons at the bottom-right, real in-game pointer, right-click map menu.

### Source commits (branch `starling`)

| Commit | Change |
| --- | --- |
| `ae314e1c4` | Removed the old five-button map panel; map actions moved to keys/wheel/touch gestures; added the right-click context menu (`Create Waypoint`, `Delete Waypoint`, `Close`). |
| `be39bb75d` | Restored only the two tiny 22px zoom buttons at the bottom-right with shared rect helpers (`map_zoom_button_rects`) used for both drawing and hit-testing; drew a hand-made crosshair cursor. |
| `fc3934169` | Moved the fullscreen-map wheel-zoom hook above the extracted scroll-normalisation block so `tests/test_mouse_scrolling_hook.py` compiles the extracted `game.rs` window. |
| `76f9be8be` | rustfmt layout for the new map code (CI formatting failure). |
| `92053296c` | Removed now-unused `CROSS`/`FLAG`/`TRASH` icon tables (CI clippy `dead_code`, warnings are errors). |
| `47b6bb965` | `#[allow(dead_code)]` on `fullscreen_map_touch` for host builds, matching the existing Android-only comment pattern used for `map_key_press`. |
| `127605daa` | Removed the hand-drawn crosshair; the map now draws the game's own pointer with `android_control_ui::draw_pointer` at the map cursor. |
| `e895ea765` | Updated the two stale `horizon-client-render` minimap tests (see below). |
| `6d124a249` | The map pointer now draws unconditionally while the map is open (the map needs it to aim waypoints); size still comes from `virtual_mouse_pointer_scale`. |

### Final map behaviour

- `M` toggles the fullscreen map; `Escape` closes the context menu first, then the map.
- While `extras.big_map` is true, `mobile_draw()` returns before drawing anything, so **no gameplay controls** (movement, attack, jump, hotbar buttons) are drawn over the map.
- Only on-map controls: two 22px buttons stacked at the bottom-right (`map_zoom_button_rects`), zoom **in** on top and **out** below, 8px from the edge, 4px apart. Both are hit-tested in `fullscreen_map_touch` (touch) and `map_mouse_click` (mouse).
- Pointer: `game/extras.rs::draw_extras` draws `android_control_ui::draw_pointer` after `extras_hud::draw_big_map` in the same HUD batch, at `extras.map_cursor`. Draw order matters: `hud()` calls `draw_touch` before `draw_extras`, so a pointer drawn in `mobile_draw` would sit underneath the map.
- The pointer follows the map cursor: touch start sets it, a left click sets it, arrow keys/WASD move it 18 gui px, and it defaults to the map centre when unset.
- Keys while the map is open: `=`/`-` (and numpad +/-) zoom, `Enter` creates a waypoint at the cursor, `Delete`/`Backspace` removes the selected/last waypoint, right-click opens the menu.
- Touch: drag pans (`map_drag`), tap positions the cursor or activates a waypoint/menu row; gestures never move the player.

### CI repair details (exact causes)

1. `tests/test_mouse_scrolling_hook.py` extracts the wheel branch between the `let (raw_x, raw_y) = match delta` marker and `// ScreenEvent.MouseScrolled.Pre`. The first wheel hook was placed **after** that marker, so the extracted fixture called a method that was not extracted. The hook now sits above the marker; keep it there.
2. Clippy (`-D warnings`) failed on `dead_code` for icon constants and for `fullscreen_map_touch`, which only the Android-only `mobile_controls` module calls. Fixes were narrow, not global allows.
3. `crates/horizon-client-render/src/minimap.rs` tests were stale from the fork's earlier map work, not from this pass:
   - `the_image_is_north_up_or_turns_with_the_player` hard-coded 128px coordinates (centre 64). It now derives the centre from `SIZE / 2` (SIZE is 1024 in this fork).
   - `tiles_fill_nearest_first_and_follow_changes` asserted that unloaded chunks drop their tiles. This fork intentionally retains explored tiles for the persistent map, so the test now asserts the tile is kept.
   These were the only two failing Rust tests; the rest of the 1325 tests passed.

### Verification — 2026-11-16

- Final source commit: `6d124a249` ("Map: always show the virtual mouse pointer while the map is open").
- Checks run `43813504214`: **success** (public-tree check, Python tests, rustfmt, clippy for workspace + server build, Rust tests).
- Android run `43813504273`: **success**, ARM64-v8a only. Artifacts: `starling-apk-arm64-v8a` ID `14202201740` (34,459,349 bytes) and `starling-symbols-arm64-v8a` ID `14202236490`. Run URL: https://github.com/DarkDevil-937/starling-rust-26.3/actions/runs/43813504273
- Earlier green build for the same behaviour: run `43811831085` on `127605daa`, APK artifact `14201361149`.
- Device test: **pending**. User has not reported results for this build.

### Known limitations / next checks

- No device verification: button size/placement, pointer size, menu placement and touch feel are approximations of Xaero, not measured on the POCO F7.
- The pointer is positioned by touch/click/keys; the touch HUD's virtual-mouse stick is hidden on the map, so the stick cannot move the pointer there. Decide with the user whether the stick should stay available.
- A documentation-only commit also triggers `android.yml`; do not mistake that run for a new source build.


## 2026-11-16 — Repair OpenCode map-input commit CI failures

- Preserved OpenCode/DeepSeek's source commit `a25d5935b33fbb6e2376f81f3adab8f94e51f8f3` and repaired it with follow-up commits; no rollback of the map interaction redesign.
- Checks #104 (run ID `43823196096`) failed in offline research tooling test `test_screen_mouse_dragged_hook.py::test_drag_lifecycle_delta_and_cancelled_cursor`. The extracted standalone Rust fixture copied the cursor-move event branch, which now calls `map_cursor_dragged`, but the fixture's fake `Game` did not define that method.
- Android #109 (run ID `43823196149`) failed compiling ARM64-v8a with Rust E0599 at `crates/horizon-client/src/game/mobile_controls.rs:299`: `map_pointer_state` is implemented on `Game`, not `ExtrasState`; the call incorrectly used `self.extras.map_pointer_state(...)`.
- Fixes: call `self.map_pointer_state(...)` from the Game implementation; add a narrow `map_cursor_dragged` stub returning false to the extracted test fixture so that test continues to isolate native mouse-drag dispatch rather than map-pan behaviour.
- Follow-up commits: `28657e689763b470ad813707933abbd895398ad8` (receiver fix), `febcfeaf5a3152d2893e938f6436068d7d38e482` (test fixture fix). Both preserve the preceding OpenCode commit as ancestry.
- These fixes are not CI-verified until the new Checks and ARM64-v8a Android workflows complete. No APK from Android #109 was produced; only ARM64-v8a is in scope.


## 2026-11-16 — Fix fullscreen-map virtual cursor touch routing

- User clarified the intended interaction: retain the launcher/game virtual-mouse cursor; move it around the map, hold left-click, and move the cursor to pan. Do not replace this with one-finger direct map dragging.
- Diagnosed the stuck-centre cursor: while playing, `mobile_touch()` routed ordinary touches through gameplay `TouchControls::update()`, then treated its joystick/camera delta as virtual-mouse movement. The menu-only `android_virtual_touch()` path—which uses `TouchUi::relative_touch()` to move the actual cursor and emits `CursorMoved` events—was never used for fullscreen-map touches.
- Fix commit: `ec11abcb949d459b4b252e55dd510f28bc5b4223` — `Route fullscreen map touches through virtual mouse`.
- In `crates/horizon-client/src/game/mobile_controls.rs`, after `android_aux_touch()` has first had the opportunity to consume launcher button touches, free-screen touches while the fullscreen map is open are now routed through `android_virtual_touch()` and returned. This preserves launcher button handling while making the real virtual cursor move. The existing `CursorMoved` event then reaches `map_cursor_dragged()`; when left-click is held, the map pans.
- This is a targeted input-path fix, not a full map redesign. No UI geometry or zoom-button size changed in this commit.
- CI and device verification: pending. Must verify Checks and ARM64-v8a Android workflow for this exact source commit, then have the user test cursor movement, left-click drag pan, gameplay-input blocking, and M/Esc close. Do not recommend an older APK as containing this fix.


## 2026-11-16 — Restore virtual-mouse clicks on the fullscreen map

- User tested the previous cursor-routing commit: the cursor now moves, but the launcher’s on-screen left/right mouse buttons and fullscreen zoom buttons did not respond to cursor clicks.
- Root cause: `android_virtual_mouse_clicks()` returned early whenever `pad_in_menu()` was false. That is correct for menu-only virtual mouse clicks, but the fullscreen map is a gameplay screen, so its cursor clicks were silently discarded. The on-screen launcher mouse buttons also flowed through the generic menu mouse dispatcher rather than the map's own click handler.
- Source commit: `0406f8e6af86f434d470e0392ea45761da66b852` — `Route virtual mouse clicks to fullscreen map`.
- File changed: `crates/horizon-client/src/game/mobile_controls.rs`.
  - `android_virtual_mouse_clicks()` now permits clicks while the fullscreen map is open even though `pad_in_menu()` is false.
  - A short virtual-cursor tap is routed to `map_mouse_click(Left, ...)`, which activates the shared map hit-testing for zoom buttons and waypoint selection.
  - A stationary long-hold is routed as right-click to `map_pointer_state`, opening/activating the map context menu.
  - `android_menu_mouse()` now routes the launcher's on-screen left/right mouse buttons directly to the map's handlers while the map is open. Left-click down/up controls map dragging; right-click opens the map context menu. These clicks no longer leak into gameplay.
- The map's existing zoom hit rectangles and 22px compact button geometry were not changed; the fix connects the virtual cursor click to the existing hit test.
- Verification: Android run #117 (ID `43834058899`) was triggered on this exact source commit; status was `in_progress` at the time of this log update. Checks workflow status must also be recorded after checking. Do not install an APK until the ARM64-v8a workflow succeeds and its artifact is confirmed.
- ABI scope: ARM64-v8a only. Device retest is pending; do not describe the click fix as device-confirmed until the user tests it.

- Small follow-up source commit: `0e7d467eeb4147d0931436e9792a59055bb98a4b` — `Keep map clicks out of text field handling`. It prevents map cursor taps from invoking the unrelated text-field/keyboard helper. This is included in the latest branch source and must be included in final CI validation.


## 2026-11-16 — Make fullscreen-map mouse controls match launcher buttons

- User clarified that map interaction must use the launcher's dedicated on-screen virtual-mouse buttons, not gameplay attack/use triggers and not direct one-finger map dragging.
- Source commit: `86a8f371d291913941dbcccfa0f0cc4cb0100f03`.
- In `crates/horizon-client/src/game/mobile_controls.rs`, fullscreen-map mode now blocks all gameplay pad state without translating attack/use triggers into map LMB/RMB. The dedicated launcher `Action::MouseButton` path remains the sole button route: LMB down/up calls the map left-click handler (hold + move cursor pans; release stops), and RMB down opens the map context menu.
- Also fixed cancelled launcher-button touches: any held virtual mouse button is explicitly released through the same map dispatcher before touch state is cancelled, preventing a stuck drag or stuck right-click state.
- The existing cursor-relative movement path remains in place. Zoom-in/out still use the compact existing hitboxes and are activated by moving the cursor over a button and pressing launcher LMB.
- Verification: CI and device test are pending for this source commit. Android target remains ARM64-v8a only. Do not install an older artifact or claim device verification until the latest ARM64 workflow succeeds and the POCO F7 test is completed.


## 2026-11-16 — Stabilize Preview APK signing across CI runs

- User reported Android install error `INSTALL_FAILED_UPDATE_INCOMPATIBLE`: the installed `io.github.juanluci.starling.preview` and new Preview APK had different signing certificates.
- Verified the Android #123 job log (run `43845378625`): `STARLING_DEBUG_KEYSTORE_BASE64` was empty (`KEYSTORE:`), so `build_apk.py` generated a fresh `target/android/debug.keystore` on the ephemeral GitHub runner. That makes each CI APK use a different certificate and prevents in-place updates.
- Workflow fix: `6e10a3612b2a67563be7d9be9e8b163c64852814` adds an `actions/cache@v4` restore/save for `target/android/debug.keystore` under stable cache key `starling-preview-signing-key-v1`. Once the first new build saves this keystore, later Preview builds on this branch reuse it. A configured `STARLING_DEBUG_KEYSTORE_BASE64` still takes precedence.
- Important one-time migration: this cannot reproduce the private signing key from an already-published APK. The new cached key will differ from the currently installed ephemeral-key build, so one uninstall/reinstall may still be required. Before uninstalling, back up `Internal storage/StarlingPreview` if it exists; the app uses this shared-storage folder for Preview game files when storage access was granted. If game files are only in app-private storage, uninstalling may delete them.
- CI/device verification of the new signing-cache workflow is pending. Do not install the artifact until its ARM64-v8a workflow succeeds and the cache-save step completes.


- Follow-up migration improvement: instead of forcing an uninstall of the old `.preview` app, the Preview variant now uses package ID `io.github.juanluci.starling.preview2` and label **Starling Preview (Stable)**. It can install alongside the old app despite the old signing certificate. `StarlingActivity.sharedFolder()` maps both package suffixes to the same shared `/storage/emulated/0/StarlingPreview` folder, so game files already in that shared folder remain reusable. Source commits: `827864706efd37eb2bd84e35a0fb3083e9aec5e8` (package/label) and `e0d9fd41cc59b709a391cef52fe323882d9f4434` (shared-folder compatibility); workflow comment updated in `1125e4ae73222f2749c440884b8d9dfb8b04427c`.
- This avoids uninstalling the old app and avoids deleting its private app data, but app-private data is not automatically shared with the new package. The shared `StarlingPreview` folder is reused when storage access was granted. The old app can be removed only after the new app has been opened and game files verified.
