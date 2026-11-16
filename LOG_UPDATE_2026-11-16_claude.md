# Log update — 2026-11-16 (Claude) — append into DEVELOPMENT_LOG.md and PROJECT_HANDOFF.md

Kept as a separate small file because the connector cannot append; whoever continues (any AI) should paste this
into both files and delete this file.

## Xaero-style fullscreen map pass 2
Commits on `starling`: 3ee2b2db1019cf29ca02aaa5c323a69092dddf6d (extras_hud.rs) and
5b21b080de3cc06291c2d4bdded996b873cb43c6 (game/extras.rs). Based on 04777e8.

Changes (source only; NOT yet CI-verified, NOT device-tested):
- `draw_big_map` takes a new `biome: &str` argument. Top-centre box now shows "X: .. Y: .. Z: .." (whole blocks) and a
  second line with the biome name (e.g. "Plains").
- `Game::map_biome_name` (game/extras.rs) looks the biome up like the debug screen does (`world.chunks` -> `chunk.biome`
  -> `res.biomes().list[..].name`), strips the `minecraft:` prefix and capitalises words. Empty when the chunk is not loaded.
- Left rail shows a pixel gear icon instead of the word "MAP" (decorative, no action yet).
- Right-edge buttons are pixel icons (close, +, -, waypoint flag, trash) via `draw_icon` (7x7 patterns, 2px cells).
- Compass is a black box with a cross and N/E/S/W.
- Cursor is now a translucent white chunk-sized square (16 blocks, snapped to the chunk grid) with a border.
- Bug fix: the buttons were drawn from y=56 but touch hit-tests used y=8 with a 6px pitch. Both now use y=60 and a
  5px pitch (`fullscreen_map_touch` in game/extras.rs), so taps match the drawn buttons.

Still missing vs the Xaero reference: richer terrain shading/detail (minimap.rs uses vanilla map colours with 4 brightness
levels), functional gear/settings, waypoint icons on the map, mob/player dots, rotate-with-view, waypoint rename.
Next: run Android + Checks workflows for 5b21b08; if Checks fail, read the exact fmt/clippy error before changing code.
