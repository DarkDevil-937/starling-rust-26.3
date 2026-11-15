# Starling project handoff

Last updated: 2026-11-15

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
