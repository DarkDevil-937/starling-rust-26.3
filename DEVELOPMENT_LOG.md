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
## Required validation checklist

For every code change:
1. Record date, branch, commit SHA/message, exact files changed, and what changed.
2. Record any known limitations or risks; distinguish implemented from planned.
3. Record the exact GitHub Actions run ID/URL and ABI; record status and conclusion only after checking.
4. Record whether the user tested it on device and the actual result. Never imply device testing happened if it did not.
5. Keep `PROJECT_HANDOFF.md` in sync with the latest state and point to this log.
6. Preserve ARM64-v8a-only builds unless the user explicitly changes that requirement.
