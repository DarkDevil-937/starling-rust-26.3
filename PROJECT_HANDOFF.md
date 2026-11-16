# Starling project handoff

Last updated: 2026-11-16

## Project identity
- Repository: https://github.com/DarkDevil-937/starling-rust-26.3
- Active branch: `starling`
- App: Starling, Rust Android port/fork of Horizon / Minecraft 26.3
- User device: POCO F7
- Required build ABI: **ARM64-v8a only** (`aarch64-linux-android`). Do not add x86, x86_64, or armeabi-v7a builds.
- User works from Android; assume no desktop/PC.

## Documentation / continuity requirement
The user explicitly requires every meaningful repository edit to be documented so another AI can continue without losing context.
- Read `DEVELOPMENT_LOG.md` first for the chronological history, implementation notes, CI results, device-test status, and next planned work.
- Update `DEVELOPMENT_LOG.md` after every meaningful code/config/workflow change and build or device-test result.
- Keep this `PROJECT_HANDOFF.md` updated with the current state, latest commit, current priority, build status, limitations, and next steps.
- Record exact commit SHAs, file paths, workflow run IDs/URLs, and whether the change is merely committed, CI-verified, or actually tested by the user.
- Do not claim a feature is fixed just because code was committed or CI passed.
- If a change is made, do not leave the next AI to infer what happened from Git history alone.

## User's current priority
Build a genuinely useful fullscreen Minecraft world map and minimap. The critical requirement now is an explored-world map:
- When a chunk has been explored/scanned, its terrain must remain visible when the player moves away, the game unloads that chunk, or the user pans the map away from the player.
- Newly loaded chunks should be added as the player explores.
- Unexplored areas should remain dark/unknown.
- The map should not be limited to the currently loaded ~30x30 chunk area.
- The user wants a polished fullscreen map, not a centered square with poor controls; zoom and pan must work without moving the player.
- Persistent cache across app restarts is a later enhancement unless user explicitly prioritizes it; the current change only retains tiles in memory during the running session.

## Next feature priority: improve map image quality
User says fullscreen map quality/resolution is very low. This is the next priority, before more decorative map features.
- Current renderer in `crates/horizon-client-render/src/minimap.rs` uses `SIZE = 512`.
- Each chunk tile contains 16×16 sampled columns.
- `compose()` generates a 512×512 RGBA image.
- `crates/horizon-client/src/extras_hud.rs` draws the map image across the fullscreen HUD viewport.
- Do not blindly increase texture size. First trace texture allocation/upload, GPU sampler filtering (nearest/linear), viewport scaling/cropping, and zoom-to-blocks-per-pixel. Determine whether blur comes from source resolution, filtering, or stretching.
- Target: crisp readable terrain at multiple zoom levels, smooth pan/zoom, and map gestures must not move the player.
- Balance sharpness against RAM/GPU memory and frame time on POCO F7.
- A first sampling improvement changed the fullscreen default from 2.0 to 1.0 blocks per pixel; a follow-up map interaction update sets the starting zoom to 0.5 blocks per pixel so terrain appears larger on screen (less world area visible; still block-level map data). Latest commits `7d9d8fb...`, `9a2e6e3...`, and `cedc058...` now also retain the in-memory map cache through title/disconnect screens, offset the player marker according to map pan, and replace chunky fullscreen controls with a restrained flat HUD. See `DEVELOPMENT_LOG.md` for exact files and limitations. Build and device test pending. Investigation confirms the 512×512 image is copied at full declared size into the `horizon/minimap` atlas slot, and `horizon/` sprites are excluded from atlas shrinking. Inspect the standard sampler's default filtering and fullscreen zoom/blocks-per-pixel before editing. Since the map stores one colour per world block, increasing SIZE alone may just display more world rather than improve detail.

## Current source state
- Latest known branch HEAD: check GitHub branch `starling` before editing; the latest documentation commit records successful run #27 and the map-quality investigation. Latest source-code commits: `13cacf9da4c46edf7980ba5214cee7e8b1c35c6d` and follow-up `b121e010167582aee202c7077a848d2484058c79` (fullscreen map default sampling detail). Check branch `starling` for any newer documentation commits.
- Latest source-code commit message: `Keep explored map chunks after unload`
- Latest changed source file: `crates/horizon-client-render/src/minimap.rs`
- Previous map work is already on branch `starling`, including:
  - `crates/horizon-client-render/src/minimap.rs`: map texture size increased from 128 to 512.
  - `crates/horizon-client/src/extras_hud.rs`: fullscreen map cover-crop to viewport, pixel-style map buttons, framed minimap and compass labels.
  - `crates/horizon-client/src/game/extras.rs`: fullscreen map touch handling, zoom/pan state, pan scale adjustment.
  - `crates/horizon-client/src/game/mobile_controls.rs`: earlier touch overlay suppression while fullscreen map is open.
- Prior known map UI commit: `8d1f790b70a6e47aeb9fd3a686231b25b3680e18`.

## Explored-chunk fix just committed
In `crates/horizon-client-render/src/minimap.rs`, `Minimap::update()` previously ran `self.tiles.retain(...)` and removed any tile if its chunk was unloaded, or outside the current radius. This directly caused explored terrain to disappear when panning or moving away.
- Commit `244385c335043745aa5efdc05d8662ca665e8226` removes that pruning.
- The map now keeps scanned tile data in `Minimap.tiles` for the lifetime of the current map/dimension session.
- The update loop still scans currently loaded chunks near the current map view. `compose()` can render from retained tiles after chunks unload.
- `clear()` still resets the cache on dimension change / map reset.
- This change is committed but **not yet build-verified or device-tested**.
- RAM grows as more chunks are explored; cache persistence and world/server identity need future review.

## Build history / current CI
- Android workflow: `.github/workflows/android.yml`
- ARM64-only workflow run for explored-chunk source commit:
  - Run #27, ID `43723719706`
  - URL: https://github.com/DarkDevil-937/starling-rust-26.3/actions/runs/43723719706
  - Final result checked on 2026-11-15: **success**. ARM64-v8a APK artifact `starling-apk-arm64-v8a`, artifact ID `14165247537`; symbols artifact ID `14166051091`.
- Documentation commits also trigger workflow runs because the workflow listens to branch pushes. Do not confuse a documentation-only run with a new source-code build.
- Previous verified run for commit `8d1f790b...`:
  - Run ID `43722047939`
  - Conclusion: success.
  - This is an older build and does not include the explored-chunk cache fix.
- Do not claim latest code builds or works until the correct source-code commit is covered by a successful ARM64 build and the user tests it.

## Known implementation details / risks
- `Minimap` owns `tiles: FxHashMap<(i32, i32), Tile>`; each tile stores 16×16 map colours, height and water depth for a chunk.
- `compose()` samples the cached tile map using world block coordinates.
- `update_minimap()` in `crates/horizon-client/src/game/extras.rs` scans within a radius derived from map reach and capped by render distance. Previously scanned tiles are now retained after unload.
- Cache currently lives in RAM only. Long-session memory growth is possible because explored tiles are intentionally not pruned. Consider a bounded/persistent disk-backed region cache in a later step, without silently discarding explored terrain.
- The world/dimension identity is represented by `minimap_dim`; dimension change clears current cache. Check world/server switching behaviour to ensure the cache is also cleared when a different world is joined in the same dimension.
- Texture size 512 is a sampled view of known cached chunk tiles, not a full world reveal.
- Do not revert earlier map UI/touch changes while working on map quality or tile persistence.

## 2026-11-15 quality investigation and map interaction update
- No source change yet. See `DEVELOPMENT_LOG.md` for details: atlas slot is not intentionally downscaled; map is a per-block colour raster; inspect default sampler and fullscreen zoom/blocks-per-pixel before deciding whether to increase resolution.
- Latest map interaction commits: `7d9d8fb...` (cache/marker), `9a2e6e3...` (fullscreen HUD), `cedc058...` (closer starting zoom). Latest ARM64 CI result and device validation still pending.
- Cache survives leaving/rejoining while app process remains alive, but is not persisted to disk; different worlds in the same dimension still need identity isolation.
- User has not yet tested a map-quality build.

## Required next actions
1. Recheck branch `starling`, workflow run #27, and any newer runs. Confirm which run actually builds source commit `244385c...`; ignore documentation-only runs for source verification.
2. Confirm workflow still builds ARM64-v8a only.
3. Once build succeeds, give user the exact APK artifact location and ask them to test:
   - Explore an area, open map, pan away from player.
   - Walk far enough that old chunks unload.
   - Reopen/pan map to the old area: explored terrain should still be visible.
   - Explore new chunks: they should be added.
   - Unexplored terrain should stay dark.
4. Next implement map image quality improvement: trace texture allocation/upload/sampler and HUD scaling before deciding whether to increase resolution. Test on POCO F7 and monitor performance.
5. Later consider persistence across app restarts (serialize explored tiles per world/dimension) and robust cache lifecycle when switching servers/worlds.
6. Update both `DEVELOPMENT_LOG.md` and this file after every meaningful change, CI result, or device test.

## User communication rules
- Be direct and casual; user often says “bro”.
- Do not ask again for device or ABI.
- Never say “fixed” before build success and user device test.
- Clearly distinguish committed code, CI success, APK availability, and device-tested behaviour.
- User prefers action and concrete repo changes over generic instructions.


## Latest map remaster — 2026-11-15

The newest source commits on `starling` add disk persistence, waypoint controls, a denser raster, and a new fullscreen HUD. The chronological record is in `DEVELOPMENT_LOG.md`.

### Implemented in source (CI/device validation pending)
- Explored chunk tiles save to `HORIZON_PRIVATE/map-cache/<world-server-dimension-key>.json`.
- Cache identity includes `world_root`, the connected server address (or `singleplayer`), and dimension. This prevents ordinary world/server/dimension collisions.
- Tiles load when entering a world, save every three seconds when dirty, and flush when leaving the world.
- W adds a waypoint at the player's current X/Z. D removes the most recently added waypoint. Waypoints persist in a sibling JSON file and render as labelled map markers.
- Fullscreen map HUD now has a full-width header, coordinates, bottom usage legend, and close/zoom/waypoint controls.
- Raster is 1024×1024 instead of 512×512; default blocks-per-texel is 0.25. This improves displayed raster density but still uses vanilla block map-colour samples, not sub-block terrain geometry.
- Map touch handling continues to capture gestures while fullscreen map is open.

### Source commit list
- `47ada6624c4c133580462cd46c2e0725224ab170`
- `a0295c1e8eab1662b19d12895924509675556d56`
- `96a34af0c6e126a53c92c0a1d0bbea8de1e93a2b`
- `27cbb731cfb492cebf73ef0c835b9c5c74554e47`
- `b8ce99588a61764830f2184e49696844f63dbd32`
- `b9e009d7cff91710661b0d97635269bb5e409875`
- `4a003bba5858e6f6ed0c51eb1a4c0c8bd87b983d`
- `224aa27e155106a65e69eb3607e39dc5185049a2`
- `ab69ee7552034fdaa8cde18ca281b1b5e588b9f9`
- `a6bb56350d318827959da70dd5f8f20fcd8dba4e`

### Known risks / remaining work
- **Not build-verified yet.** Confirm ARM64-v8a CI for the latest source before offering an APK.
- 1024×1024 composition/upload is 4× the previous pixel workload; measure frame pacing and memory on POCO F7.
- Waypoints currently auto-name, add at current player location, and remove only the last one. Renaming and selecting/deleting arbitrary markers remain future work.
- Map raster samples block map colours; no sub-block geometry or high-detail terrain shading yet.
- Verify the world-root/server-address identity against both singleplayer saves and remote servers in device testing.
- Do not call this fixed until CI passes and the user confirms on device.

### Immediate next actions
1. Check latest branch head and GitHub Actions runs after these commits. Diagnose/fix compile and formatting failures.
2. Keep build target ARM64-v8a only.
3. Once CI succeeds, provide the exact new APK artifact.
4. Ask the user to test: explore chunks, leave and force-close/reopen Starling, rejoin; verify terrain returns; add/remove waypoints; switch worlds/servers/dimensions; test drag/zoom and confirm player movement is unaffected.
5. Update this handoff and `DEVELOPMENT_LOG.md` with exact CI run and test outcomes.

- Follow-up source commits: `641bd654021fdda9dd13c50abf5fb25aff4ef5d2` (cache key comparison and safe waypoint-file rename), `f30aa07639d7f445780db5ef3a561d5cacaa5e95` (cache round-trip unit test), and `b105d832c65282838ba595e6f0f7452e5261667b` (portable test filename). These are included in the current branch state; CI still needs to validate them.

- Waypoint interaction follow-up: `edd7986391bdf8dfe6d9efefdfadbbd0d23562cf`, `17cc3e7407dd2235c9ca9aed17c03af12153e48b`, and `2e87e1ec1ecc4b9d65421ff6e72dfa89f40b753d` add tap-to-select and selected-marker highlighting; D deletes the selected waypoint, or the last waypoint when none is selected. Auto-generated names remain; user rename/edit is not implemented.

- Final source-format follow-ups: `116c662fb9dabd0d6b7e14a2a26e36024787078b`, `203aaa70efeb6f845333b092dad847407fb163fb`, `7cf868d9c255ad24d836bfa47cf5d13f2b0cabce`, `83837928fb9bbac20a8224b91da9171b39a06599`, and `bdd807d3ada5acc91316937855589956763b74f7`. These formatting-only commits are included in the branch head; validation is still pending.

- Zoom-range follow-up: `51bbb7962e36a98c76c4e568431a960a8018e2da` aligns the zoom-out minimum with the zoom-in minimum (0.125 blocks per texel).

- Rustfmt follow-up: `fc2f64a0607276413c2af18f415bee7421a856b7` formats the cache timestamp helper; validation remains pending.

 
## Workflow cleanup — 2026-11-15

Removed seven upstream-only workflows whose triggers/guards target `horizon-rust/horizon` or `main`, not this Starling fork's active `starling` branch. Kept the Android ARM64 APK workflow, Rust checks, PR cancellation helper, native-mod build workflow, and manual pack benchmark. See `DEVELOPMENT_LOG.md` for the exact file list. This does not delete historical Actions runs or artifacts.

 
## CI failure diagnosis — 2026-11-15

The failed checks run #72 (SHA `d3eefa4f`) failed in the formatting/lint step due to rustfmt differences in `extras_hud.rs`, `game/extras.rs`, and `minimap.rs`. This was not an Android NDK or ARM64 compiler failure. The three files have since been formatted in commits `471eab6e5099ad954fa6db4533e94ca1ab09c911`, `c9d7444b932f063254472f6af5b2479e8bf9c8d7`, and `e4dfe0f420503241450053fc25d2d38fff9efc2e`. Recheck workflows #74/#75 were started; wait for the latest run on the current branch head. ARM64 run #76 is still running on an earlier commit. Android runs cancelled by later pushes are concurrency cancellations, not failures. Current build must be validated against the latest SHA before sharing an APK.


## Workflow/build repair — 2026-11-15

Current workflow set: android.yml (ARM64-v8a only), ci.yml, native-mods.yml. Removed pack-bench.yml (upstream-only benchmark) and cancel-on-close.yml (PR housekeeping not required for the normal build flow). Checks #78 exposed three workflow-presence tests for deleted files and one test requiring three self-hosted jobs despite the consolidated single-job CI design; those assertions now match the retained functional tooling and single-job checks. Android #82 found serde's unsupported 256-element arrays and a missing lifetime in the serialized cache tuple. Tile storage now uses vectors with strict 256-entry validation, and serialized keys are copied. Await fresh CI and ARM64 results before calling this build successful.

## Latest requested map interaction refinement — 2026-11-15

User reports explored chunks persist after restarting and map quality is now acceptable. Remaining requirements: compact left-side world/coordinates panel, waypoint placement anywhere on the map, and a Xaero-like virtual cursor that can navigate the map and activate controls.

- Source changes committed in d3595a395a7746e09bf46c83d924653b9ef7c11d, with follow-up formatting/borrow-safety commit 8e282c8d69dac644632ab0652e2d56cb1f3b45f2:
  - extras_hud.rs: compact left info card; crosshair cursor; cursor world-coordinate readout; fullscreen terrain remains edge-to-edge.
  - game/extras.rs: tap to position cursor; Add waypoint uses cursor-to-world conversion rather than player position; arrow keys move cursor; Enter activates the control beneath the cursor.
- Latest verified build before this update: Android run #83 (43735462980), ARM64-v8a APK artifact 14170610432, success. Checks #79 (43735462920) failed only due to rustfmt differences.
- Workflow run deletion is not available through the current GitHub connection. No historical runs have been deleted. Active workflows remain android.yml, ci.yml, and native-mods.yml.
- Latest source-code commit: 8e282c8d69dac644632ab0652e2d56cb1f3b45f2. Latest branch HEAD is documentation-only commit 48f2e0775212f66ca41966b6bac4b62d77dff638.
- Latest validation: checks #81 (43737709898) in progress and Android #85 (43737709847) pending for the latest SHA. Earlier #80/#84 are still running on the previous cursor commit, not the final code.
- Next: wait for checks #81 and ARM64 build #85, fix any errors, then device-test cursor placement, buttons, pan/zoom, waypoint persistence, and player-input isolation.


## CI repair note — 2026-11-15

- Latest checks run #83 failed at `cargo +stable fmt --all -- --check`; the log identified formatting-only differences in `extras_hud.rs` and `game/extras.rs`. Rust clippy and tests were not reached in that run.
- Applied formatting corrections matching the CI diff. Android run #87 was building ARM64-v8a for the same source code at the time of inspection; no success claim until its artifact and final checks are confirmed.
- Always inspect workflow job logs after failures, fix the specific root cause, and rebuild before asking the user to install. User needs the ARM64-v8a APK artifact from the final successful run, not an arbitrary source commit.


## Current authoritative status — 2026-11-15 (after run #88)

- Latest source commit at inspection: `8bbd72d00879caedbe98aadefa1b79c1382feccb`.
- Android run #88, ID `43738219528`, succeeded for **ARM64-v8a only**. APK artifact `starling-apk-arm64-v8a`, ID `14171324177`; symbols artifact ID `14171319351`. Run URL: https://github.com/DarkDevil-937/starling-rust-26.3/actions/runs/43738219528
- Checks run #84, ID `43738219505`, failed at rustfmt due to one remaining line-wrap in `crates/horizon-client/src/game/extras.rs`; Clippy and Rust tests were skipped. Exact rustfmt correction has now been applied; wait for a new checks run before calling full validation complete.
- Do not ask the user to install yet. Prefer the APK from the next successful Android run on the post-format-fix commit after checks also pass.
- User has not device-tested the virtual cursor/waypoint build. After a verified build, test tap-to-position, add waypoint away from player, arrow-key cursor navigation, Enter activation, map controls without moving player, and cache persistence.
- Historical Actions run deletion is not supported by the currently connected GitHub actions; no run history was deleted.


## Latest CI diagnosis — 2026-11-15 (run #89)

- Current source commit before lint repair: `842f7c06bf747ff1c9a9ed75b8bc4bc8a801e515`.
- Checks #85 (ID `43739638523`) passed formatting and Python tests, then failed compiling/linting because `Game::fullscreen_map_open` is only called by the Android touch HUD, which is not compiled in the host checks target; warning is denied as an error. The Android target build #89 succeeded on this source, corroborating the method's Android use.
- Narrow fix applied: document Android-only use and add `#[allow(dead_code)]` to that one method only. Do not suppress dead-code warnings globally.
- Wait for checks and ARM64 APK build on the new lint-fix commit before telling the user to install. User device test is still pending.


## CI follow-up — unused accessor lint (2026-11-15)

- Latest inspected commit: `842f7c06bf747ff1c9a9ed75b8bc4bc8a801e515`.
- Android #89 / run ID `43739638532` succeeded; ARM64-only APK artifact `starling-apk-arm64-v8a`, ID `14171568404`. Run: https://github.com/DarkDevil-937/starling-rust-26.3/actions/runs/43739638532
- Checks #85 / run ID `43739638523` failed during Rust lint/build on `fullscreen_map_open` being unused (warnings-as-errors). Formatting and Python checks had passed; Rust core tests did not run.
- Removed the unused accessor from `crates/horizon-client/src/game/extras.rs`; no callers were found. This does not change the fullscreen map state machine.
- Current build is not yet recommended for installation; wait for green checks and a successful ARM64 build on the cleanup commit. Device testing remains outstanding.
- Historical Actions run deletion remains unavailable through the current GitHub connection.


## 2026-11-15 follow-up: repair failed #91/#87 builds

- Failed Android #91 identified the remaining call to removed `fullscreen_map_open()` in `crates/horizon-client/src/game/mobile_controls.rs`. Replaced it with `self.extras.big_map`; the map state is the same field already used by the fullscreen-map implementation.
- Failed Checks #87 identified an unrelated host-only `unreachable_code` error in `crates/horizon-client-render/src/audio.rs`: without the `audio` feature, `Stream` is `Infallible`, making the successful match arm unreachable. The stream-opening/match path is now compiled only with `feature = "audio"`; no-audio builds initialise `(None, false)`. Audio-enabled Android behaviour is retained.
- Changes are committed but not yet build-verified. Only install a new ARM64-v8a artifact after the matching Android workflow succeeds and checks are green. Device validation remains outstanding.


## CI repair — 2026-11-15 (runs #92 and #88)

- Inspected failed Android run #92 (job 132896721038) and Checks run #88 (job 132896720784) for commit `fa6c258d70213409a25b81d141334d3b81b1ce87`.
- Android's exact compiler error was E0616: `mobile_controls.rs` reads `ExtrasState.big_map`, but the field was private to the sibling `extras` module. Narrowly changed the field to `pub(super)`, exposing it within the parent `game` module only.
- Checks' exact error was `dead_code` on the no-audio `Audio::open_named` stub. That stub has no callers because `set_device` is separately compiled for no-audio builds. Removed the unused stub; the real `open_named` remains compiled only with `feature = "audio"`.
- These are source-level corrections to the two actual errors; fresh checks and the ARM64-v8a Android build are required before considering the repair validated. No APK from #92 was produced.


## Latest UI request — Xaero-style fullscreen map

- User-provided reference shows a full-bleed terrain map, narrow black left rail, compact X/Y/Z + biome strip at top centre, compass at top right, small right-side map controls, and bottom zoom indicator.
- A visual-only pass is being implemented in `crates/horizon-client/src/extras_hud.rs`: left rail and waypoint count, centred coordinate panel, compass, smaller controls and zoom label. Existing pan/zoom/cursor/waypoint behaviour and five control hitboxes are intended to remain intact.
- Biome name and a functional settings menu are not implemented because the current draw call does not supply biome data and no settings action exists.
- Current baseline: Android #93 succeeded for ARM64-v8a on `274988e4a3dcb84e2cc58e5d984c0df7b26a0c45`; Checks #89 failed two minimap tests. This new UI source must be separately checked and built. No device test of the reference-style UI yet.


## 2026-11-16 — Workflow #94/#90 result and follow-up

- Commit `f09f8ed55c1c2eabe616210e6f93eddd838d1d2f`: Android #94 succeeded; confirmed artifact `starling-apk-arm64-v8a` exists (34,459,570 bytes). Checks #90 failed at formatting check only.
- Exact failure: `cargo +stable fmt --all -- --check` reported three formatting hunks in `crates/horizon-client/src/extras_hud.rs`; Clippy and Rust correctness tests were skipped because the formatting step stopped the job.
- Applied rustfmt's exact requested layout to those three areas. No functional changes intended.
- New commit should trigger Checks and ARM64-v8a Android workflows. Verify both fresh run conclusions and artifact list before recommending the new build. The #94 APK belongs to the previous source SHA and is not the formatting-fix commit.


## Current authoritative status — 2026-11-16 (Xaero-like map screen)

Read this section first; the sections above are historical.

- Latest source + docs commit before this note: `6d124a249` — "Map: always show the virtual mouse pointer while the map is open".
- Branch: `starling`. Remote: `origin` = `DarkDevil-937/starling-rust-26.3`. `upstream/starling` is starling-client/starling; always pass `-R DarkDevil-937/starling-rust-26.3` to `gh run` commands or they resolve against upstream and 404.
- Checks run `43813504214`: **success** (public tree, Python tests, rustfmt, clippy workspace + server build, Rust tests).
- Android run `43813504273`: **success**, ARM64-v8a only.
  - APK: `starling-apk-arm64-v8a`, artifact ID `14202201740` (34,459,349 bytes).
  - Symbols: `starling-symbols-arm64-v8a`, ID `14202236490`.
  - URL: https://github.com/DarkDevil-937/starling-rust-26.3/actions/runs/43813504273
- Device test: **pending**. Nothing in this pass is device-verified.

### What the fullscreen map is now

- No gameplay controls are drawn over the map: `mobile_draw()` in `crates/horizon-client/src/game/mobile_controls.rs` returns early when `self.extras.big_map` is true.
- The only on-map buttons are two 22px zoom buttons at the bottom-right (`extras_hud::map_zoom_button_rects`), zoom in above zoom out. The same helper is used for drawing and for touch/mouse hit-testing, so they cannot drift apart.
- The cursor is **not** a new drawing: `crates/horizon-client/src/game/extras.rs::draw_extras` draws the game's existing virtual mouse pointer (`android_control_ui::draw_pointer`) at `extras.map_cursor`, after `draw_big_map` in the same HUD batch. `hud()` calls `draw_touch` first, so the pointer must be drawn from the extras pass to appear above the map.
- Right-click (and a tap on an existing waypoint) opens the map context menu: Create Waypoint / Delete Waypoint / Close.
- Keys: `M` toggle, `Esc` close (menu first), WASD/arrows move the cursor 18px, wheel and `=`/`-` zoom, `Enter` waypoint at cursor, `Delete`/`Backspace` remove waypoint.
- Touch: tap positions cursor / activates menu rows and waypoints, drag pans. Map gestures never move the player.

### Constraints learned this pass (do not regress)

- The fullscreen-map wheel hook in `game.rs` must stay **above** the `let (raw_x, raw_y) = match delta` line: `tests/test_mouse_scrolling_hook.py` extracts the branch from that marker, and a hook below it breaks the test compile.
- Clippy runs with `-D warnings`. Android-only helpers called from `mobile_controls` (e.g. `fullscreen_map_touch`) need `#[allow(dead_code)]` plus the existing comment, not a crate-wide allow. Removing map buttons also means removing their icon tables.
- `crates/horizon-client-render/src/minimap.rs` in this fork uses `SIZE = 1024` and deliberately keeps explored tiles after an unload. Tests that assume 128px coordinates or tile pruning are stale; two were corrected in `e895ea765`.
- rustfmt is not installed in the agent environment; formatting must be verified through the `checks` workflow diff (`gh run view -R DarkDevil-937/starling-rust-26.3 <run> --log-failed`).

### Next steps

1. User installs the artifact above and reports specifically: pointer size/visibility, the two zoom buttons' size and spacing, menu position, and whether any gameplay controls still appear on the map.
2. Adjust exact geometry values from that feedback rather than redesigning again.
3. Open question for the user: should the virtual-mouse stick stay on screen while the map is open so the pointer can be moved without touching the map directly?
4. Keep `DEVELOPMENT_LOG.md` updated with every change, run ID, and device result; the user requires full written process.


## Current handoff — OpenCode map-input integration repair (2026-11-16)

- Preserve the OpenCode/DeepSeek map redesign commit `a25d5935b33fbb6e2376f81f3adab8f94e51f8f3`; do not revert it.
- CI for that commit: Checks #104 / run `43823196096` failed only in the extracted offline Python/Rust mouse-drag fixture because the newly inserted `self.map_cursor_dragged(previous)` call was absent from its fake `Game`. Android #109 / run `43823196149` failed ARM64-v8a compilation with E0599 because `map_pointer_state` is a `Game` method, but `mobile_controls.rs` called it through `self.extras`.
- Applied narrow fixes: `mobile_controls.rs` now calls `self.map_pointer_state(...)`; `tests/test_screen_mouse_dragged_hook.py` adds a fixture-only stub returning false, keeping the test focused on native mouse drag dispatch.
- Repair commits: `28657e689763b470ad813707933abbd895398ad8`, `febcfeaf5a3152d2893e938f6436068d7d38e482`, plus this handoff/log documentation commit.
- Current branch head should be checked before further writes. Fresh Checks and Android workflows must both pass; Android must publish exact artifact `starling-apk-arm64-v8a` before recommending installation. Never build/recommend other ABIs. Device test remains pending.


## Latest map cursor input fix — 2026-11-16

- Latest source commit: `ec11abcb949d459b4b252e55dd510f28bc5b4223` — `Route fullscreen map touches through virtual mouse`.
- Documentation commit: `fab2974ec25eb437f754df6c2f1454076e64791c`.
- User's confirmed intended map interaction: **keep the virtual mouse cursor**. Move the cursor around the map; hold left-click and move the cursor to pan. Do not switch to one-finger direct map panning.
- Root cause: in fullscreen map gameplay, free-screen touches went through gameplay `TouchControls::update()`; its joystick/camera delta was being used as pointer movement. The proper `android_virtual_touch()` / `TouchUi::relative_touch()` path was only used while in menus, so the map cursor could stay stuck at its centre spawn point.
- Fix: after `android_aux_touch()` gets first chance to process launcher button touches, free-screen touches while map is open now go through `android_virtual_touch()`. This updates `self.cursor` and dispatches `CursorMoved`, which feeds the existing left-held `map_cursor_dragged()` panning logic. Launcher buttons remain available because auxiliary button handling runs first.
- This commit does not yet change zoom-button dimensions or overall map styling.
- CI status: **not yet verified** for this source commit. Check fresh Checks + Android workflows. Android must be ARM64-v8a only and publish `starling-apk-arm64-v8a`.
- Device test still required on POCO F7:
  1. Open map; move virtual cursor away from centre in all directions.
  2. Hold launcher left-click/Attack and move cursor; map should pan continuously.
  3. Confirm player does not move, look, break or place blocks while map is open.
  4. Confirm M and Esc close map.
  5. Check compact zoom buttons still respond.
- Next priority after input fix is device feedback and a separate small Xaero-style zoom-button sizing pass; do not combine unrelated layout changes before cursor behaviour is verified.


## Latest map click fix — 2026-11-16

- Source commit: `0406f8e6af86f434d470e0392ea45761da66b852` — `Route virtual mouse clicks to fullscreen map`.
- User-confirmed regression after the previous cursor fix: cursor movement works, but launcher LMB/RMB buttons and map zoom buttons do not respond to cursor clicks.
- Diagnosis: virtual mouse click dispatch was gated behind `pad_in_menu()`; the fullscreen map is in `Screen::Playing`, so the click path returned before dispatch. Launcher mouse-button actions also needed an explicit map route.
- Fix in `crates/horizon-client/src/game/mobile_controls.rs`:
  - Allow virtual cursor clicks while the fullscreen map is open.
  - Short cursor tap -> map left-click hit test (zoom buttons and waypoint selection).
  - Stationary hold -> map right-click/context menu.
  - Launcher on-screen LMB/RMB buttons -> direct map left/right handlers; LMB down/up starts/stops map drag and RMB opens the map context menu.
  - Gameplay input remains isolated while map is open.
- The compact 22px zoom button geometry is unchanged; this commit makes the existing hitboxes reachable by the virtual cursor.
- Android ARM64-v8a run #117 / ID `43834058899` was in progress when this handoff was written. Checks and Android run outcomes must be checked before declaring completion. Do not install an earlier APK; only use the artifact from a successful ARM64-v8a run containing this commit.
- Device validation still required on POCO F7: test cursor tap on zoom-in and zoom-out; launcher LMB hold + cursor movement pans; launcher RMB opens context menu; verify gameplay controls do not break/place/move while map is open.
- See `DEVELOPMENT_LOG.md` for the dated implementation and CI record.

- Follow-up source commit `0e7d467eeb4147d0931436e9792a59055bb98a4b` also prevents map taps from invoking the text-field click/keyboard path. Validate this latest source SHA, not the earlier click-fix commit alone.


## Latest launcher-button map input correction — 2026-11-16

- Source commit: `86a8f371d291913941dbcccfa0f0cc4cb0100f03`.
- Fullscreen map no longer interprets gameplay attack/use triggers as map left/right clicks. Gameplay pad state is blocked while the map is open; only the launcher's dedicated on-screen virtual-mouse buttons dispatch map clicks.
- Launcher LMB down/up routes to `map_mouse_click`: holding LMB while moving the virtual cursor pans continuously; releasing stops the pan. Cursor over zoom +/- then launcher LMB activates the existing compact zoom hitboxes.
- Launcher RMB down/up routes to `map_pointer_state`: the down edge opens the small context menu once; release does not activate it again.
- Cancelled button touches now explicitly release all held virtual mouse buttons before cancelling touch state, avoiding a stuck map drag/menu state.
- Cursor movement remains on the relative virtual-mouse path; no one-finger direct map drag was introduced.
- `DEVELOPMENT_LOG.md` contains the implementation record. Validate Checks and the ARM64-v8a-only Android workflow on the latest branch head before offering an APK. Device verification on POCO F7 is still pending.


## Preview APK update-signature issue — 2026-11-16

- User's install dialog shows `INSTALL_FAILED_UPDATE_INCOMPATIBLE` for package `io.github.juanluci.starling.preview`.
- Root cause confirmed from Android run #123 (`43845378625`) logs: the `STARLING_DEBUG_KEYSTORE_BASE64` Actions secret was empty. Each clean GitHub runner therefore generated a new `target/android/debug.keystore`, so successive Preview APKs had different signing certificates.
- Fix commit: `6e10a3612b2a67563be7d9be9e8b163c64852814`. `.github/workflows/android.yml` now uses `actions/cache@v4` to persist `target/android/debug.keystore` under `starling-preview-signing-key-v1`; later builds reuse the same signing key. A configured debug-keystore secret overrides the cached fallback.
- This does not recover the previous ephemeral private key. The first install after this change may still require uninstalling the old Preview app once. Preserve game files first: check/backup `Internal storage/StarlingPreview` (the app's shared Preview folder when storage permission was granted). Do not assume app-private files survive uninstall.
- Wait for successful ARM64-v8a build and successful cache save before using the new APK. Subsequent Preview APK updates should install over it without uninstalling.


- To avoid forcing the user to uninstall the old app (and losing app-private files), the Preview variant was migrated to package ID `io.github.juanluci.starling.preview2`, label **Starling Preview (Stable)**. It installs alongside the old `.preview` package despite the signing mismatch.
- Both `.preview` and `.preview2` resolve to the same shared-storage directory `StarlingPreview`, preserving access to game files there when storage permission is granted. App-private preferences/files do not transfer automatically. Keep the old app installed until the new build is tested.
- Package migration commits: `827864706efd37eb2bd84e35a0fb3083e9aec5e8`, `e0d9fd41cc59b709a391cef52fe323882d9f4434`, workflow comment `1125e4ae73222f2749c440884b8d9dfb8b04427c`.


## Fullscreen-map virtual cursor pan fix — 2026-11-16

- Fix commit: `3d41ac8437e9977f2112043491e30ca61b15ccc7`.
- Fullscreen map free-screen movement goes through `android_virtual_touch()`. Previously only `android_touch()` called `map_cursor_dragged()`, so the virtual cursor could move while held launcher LMB failed to pan.
- `android_virtual_touch()` now passes its previous cursor position to `map_cursor_dragged()` after each cursor update while the fullscreen map is open. The map's existing held-left-button and no-context-menu guards control when panning occurs.
- Launcher LMB/RMB dispatch and map zoom hit testing are unchanged. No direct finger-drag behavior added. Test latest ARM64-v8a build on POCO F7 before calling this device-verified.


## Launcher HUD input while fullscreen map is open — 2026-11-16

- Root cause: `android_aux_touch()` resolved HUD actions using only `pad_in_menu()`. That is false during the in-game fullscreen map, so the launcher's own on-screen LMB/RMB and key actions were not recognized. The map-specific click handlers were therefore never reached.
- Fix in `mobile_controls.rs`: treat fullscreen-map mode as an enabled HUD-action context, while still keeping gameplay pad input blocked. Route on-screen key actions through `map_key_press()` during the map so they cannot trigger gameplay.
- LMB press/release is routed to map drag/zoom handling; RMB press opens the context menu. Cursor movement remains on the virtual-mouse path.
- Await CI and POCO F7 testing before marking this fixed.
