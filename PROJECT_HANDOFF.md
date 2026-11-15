# Starling project handoff

Last updated: 2026-11-15

## Project identity
- Repository: https://github.com/DarkDevil-937/starling-rust-26.3
- Active branch: `starling`
- App: Starling, Rust Android port/fork of Horizon / Minecraft 26.3
- User device: POCO F7
- Required build ABI: **ARM64-v8a only** (`aarch64-linux-android`). Do not add x86, x86_64, or armeabi-v7a builds.
- User works from Android; assume no desktop/PC.

## User's current priority
Build a genuinely useful fullscreen Minecraft world map and minimap. The critical requirement now is an explored-world map:
- When a chunk has been explored/scanned, its terrain must remain visible when the player moves away, the game unloads that chunk, or the user pans the map away from the player.
- Newly loaded chunks should be added as the player explores.
- Unexplored areas should remain dark/unknown.
- The map should not be limited to the currently loaded ~30x30 chunk area.
- The user wants a polished fullscreen map, not a centered square with poor controls; zoom and pan must work without moving the player.
- Persistent cache across app restarts is a later enhancement unless user explicitly prioritizes it; the current change only retains tiles in memory during the running session.

## Current source state
- Latest known branch HEAD: `244385c335043745aa5efdc05d8662ca665e8226`
- Latest commit message: `Keep explored map chunks after unload`
- Latest changed file: `crates/horizon-client-render/src/minimap.rs`
- Previous map work is already on branch `starling`, including:
  - `crates/horizon-client-render/src/minimap.rs`: map texture size increased from 128 to 512.
  - `crates/horizon-client/src/extras_hud.rs`: fullscreen map cover-crop to viewport, pixel-style map buttons, framed minimap and compass labels.
  - `crates/horizon-client/src/game/extras.rs`: fullscreen map touch handling, zoom/pan state, pan scale adjustment.
  - `crates/horizon-client/src/game/mobile_controls.rs`: earlier touch overlay suppression while fullscreen map is open.
- Prior known commit before explored-tile fix: `8d1f790b70a6e47aeb9fd3a686231b25b3680e18`.

## Explored-chunk fix just committed
In `crates/horizon-client-render/src/minimap.rs`, `Minimap::update()` previously ran `self.tiles.retain(...)` and removed any tile if its chunk was unloaded, or outside the current radius. This directly caused explored terrain to disappear when panning or moving away.
- Commit `244385c335043745aa5efdc05d8662ca665e8226` removes that pruning.
- The map now keeps scanned tile data in `Minimap.tiles` for the lifetime of the current map/dimension session.
- The update loop still scans currently loaded chunks near the current map view. `compose()` can render from retained tiles after chunks unload.
- `clear()` still resets the cache on dimension change / map reset.
- This change is committed but **not yet build-verified or device-tested**.

## Build history / current CI
- Android workflow: `.github/workflows/android.yml`
- Latest build triggered by explored-chunk commit:
  - Run ID: `43723719706`
  - URL: https://github.com/DarkDevil-937/starling-rust-26.3/actions/runs/43723719706
  - At handoff update time it was queued; check current status before asking user to install.
- Previous verified run for commit `8d1f790b...`:
  - Run ID: `43722047939`
  - Conclusion: success.
  - This is an older build and does not include the explored-chunk cache fix.
- Do not claim the latest code builds or works until run `43723719706` (or a newer run for current HEAD) succeeds and the user tests it.

## Known implementation details / risks
- `Minimap` owns `tiles: FxHashMap<(i32, i32), Tile>`; each tile stores 16x16 map colours, height and water depth for a chunk.
- `compose()` samples the cached tile map using world block coordinates.
- `update_minimap()` in `crates/horizon-client/src/game/extras.rs` scans within a radius derived from map reach and capped by render distance. Previously scanned tiles are now retained after unload.
- Cache currently lives in RAM only. Long-session memory growth is possible because explored tiles are intentionally not pruned. Consider a bounded/persistent disk-backed region cache in a later step, without silently discarding explored terrain.
- The world/dimension identity is represented by `minimap_dim`; dimension change clears current cache. Check world/server switching behaviour to ensure the cache is also cleared when a different world is joined in the same dimension.
- Texture size 512 is not equivalent to a true large-region persistent map by itself.
- The 512 texture is a sampled view of known cached chunk tiles, not a full world reveal.
- Do not revert earlier map UI/touch changes while working on tile persistence.

## Required next actions
1. Check branch HEAD and workflow run `43723719706`. If failed, inspect job logs and fix only the actual failure. If queued/in progress, wait/check again.
2. Confirm workflow still builds ARM64-v8a only.
3. Once build succeeds, tell user the APK artifact location and ask them to test this exact behaviour:
   - Explore an area, open map, pan away from player.
   - Walk far enough that old chunks unload.
   - Reopen/pan map to the old area: explored terrain should still be visible.
   - Explore new chunks: they should be added.
   - Unexplored terrain should stay dark.
4. If it works, next consider persistence across app restarts (serialize explored tiles per world/dimension) and proper cache lifecycle when switching servers/worlds.
5. Update this file after every meaningful code change/build/test, keeping exact commit SHAs and workflow IDs.

## User communication rules
- Be direct and casual; user often says “bro”.
- Do not ask again for device or ABI.
- Never say “fixed” before build success and user device test.
- Clearly distinguish committed code, CI success, APK availability, and device-tested behaviour.
- User prefers action and concrete repo changes over generic instructions.
