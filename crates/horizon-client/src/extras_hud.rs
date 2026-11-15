//! HUD boxes in the corners: the minimap's place and mods' panels, small
//! boxes in GUI pixels stacked per corner (`docs/extras.md`).
use crate::{
    extras::Corner,
    hud::{HudAtlas, HudBatch, Rect, rgba},
};

/// The gap between an overlay and the screen edge or the next overlay.
const MARGIN: f32 = 2.0;
const BACKGROUND: u32 = 0x7800_0000;

/// Where the next overlay goes in each corner.
pub struct Corners {
    width: f32,
    height: f32,
    /// Distance from the corner's top or bottom edge, per `Corner::ALL`.
    used: [f32; 4],
}
impl Corners {
    /// `top_right` reserves the status effect icons' rows.
    pub fn new(width: f32, height: f32, top_right: f32) -> Self {
        Self {
            width,
            height,
            used: [0.0, top_right, 0.0, 0.0],
        }
    }
    /// The top-left position of a `w` x `h` box in `corner`, below (or
    /// above) what the corner already holds.
    pub fn place(&mut self, corner: Corner, w: f32, h: f32) -> (f32, f32) {
        let i = Corner::ALL.iter().position(|c| *c == corner).unwrap_or(0);
        let edge = self.used[i] + MARGIN;
        self.used[i] = edge + h;
        let x = if corner.left() {
            MARGIN
        } else {
            self.width - MARGIN - w
        };
        let y = if corner.top() {
            edge
        } else {
            self.height - edge - h
        };
        (x, y)
    }
}

/// How the minimap shows: its corner and side in GUI pixels, its image (an
/// atlas rect), the player arrow (the map item's decoration sprite) turned
/// `arrow` degrees clockwise from up, and the rim's N toward `north` when the
/// map turns.
pub struct MinimapLook {
    pub corner: Corner,
    pub size: f32,
    pub image: Rect,
    pub arrow: f32,
    pub north: Option<(f32, f32)>,
}

/// Starling big-map overlay: button size and the three button rects returned
/// so touches can hit-test them (X close, + zoom in, - zoom out).
pub const MAP_BTN: f32 = 36.0;

pub fn draw_minimap(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    corners: &mut Corners,
    look: &MinimapLook,
) {
    let (x, y) = corners.place(look.corner, look.size, look.size);
    draw_map_at(batch, atlas, look, (x, y), 4.0);
}

/// Starling: the fullscreen map (the M key), centred over a dimmed screen,
/// with Xaero-style close / zoom buttons and coordinates.
/// Returns the three button rects `[x, y, w, h]`: close, zoom-in, zoom-out.
pub fn draw_big_map(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    look: &MinimapLook,
    coords: &str,
) -> [[f32; 4]; 3] {
    batch.fill(atlas, 0.0, 0.0, batch.width, batch.height, BACKGROUND);
    let x = (batch.width - look.size) / 2.0;
    let y = (batch.height - look.size) / 2.0;
    draw_map_at(batch, atlas, look, (x, y), 8.0);

    // Coordinates under the map (Xaero style).
    let tw = HudBatch::text_width(atlas, coords);
    let cx = (batch.width - tw) / 2.0;
    let cy = y + look.size + 6.0;
    batch.text(atlas, coords, cx.round(), cy.round(), rgba(255, 255, 255, 255), true);

    // Buttons: top-right X, then + and - below it.
    let pad = 8.0;
    let bx = batch.width - pad - MAP_BTN;
    let close = [bx, pad, MAP_BTN, MAP_BTN];
    let zoom_in = [bx, pad + MAP_BTN + 6.0, MAP_BTN, MAP_BTN];
    let zoom_out = [bx, pad + 2.0 * (MAP_BTN + 6.0), MAP_BTN, MAP_BTN];

    draw_map_button(batch, atlas, close, "X");
    draw_map_button(batch, atlas, zoom_in, "+");
    draw_map_button(batch, atlas, zoom_out, "-");

    [close, zoom_in, zoom_out]
}

fn draw_map_button(batch: &mut HudBatch, atlas: &HudAtlas, r: [f32; 4], label: &str) {
    let [x, y, w, h] = r;
    batch.fill(atlas, x, y, w, h, 0xCC_1A1A1A);
    batch.fill(atlas, x + 1.0, y + 1.0, w - 2.0, h - 2.0, 0xCC_3A3A3A);
    let tw = HudBatch::text_width(atlas, label);
    let tx = x + (w - tw) / 2.0;
    let ty = y + (h - 8.0) / 2.0 - 1.0;
    batch.text(atlas, label, tx.round(), ty.round(), rgba(255, 255, 255, 255), true);
}

/// The map image at `(x, y)` with the player arrow (`h` GUI pixels from its
/// centre to its edge) and, when the map turns, the rim's N.
fn draw_map_at(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    look: &MinimapLook,
    (x, y): (f32, f32),
    h: f32,
) {
    let MinimapLook {
        size,
        image,
        arrow,
        north,
        ..
    } = *look;
    batch.rect_uv(x, y, size, size, image, u32::MAX);
    let (cx, cy) = (x + size / 2.0, y + size / 2.0);
    if let Some(r) = atlas.sprites.get("map/decorations/player").copied() {
        // MapRenderer's decorations draw the sprite flipped: its arrow
        // points down the texture, so up on screen is half a turn.
        let (s, c) = (arrow + 180.0).to_radians().sin_cos();
        let p = |dx: f32, dy: f32| [cx + dx * c - dy * s, cy + dx * s + dy * c];
        batch.quad(
            [p(-h, h), p(h, h), p(h, -h), p(-h, -h)],
            [[r.u0, r.v0], [r.u1, r.v0], [r.u1, r.v1], [r.u0, r.v1]],
            u32::MAX,
        );
    }
    if let Some((nx, ny)) = north {
        let reach = size / 2.0 - 6.0;
        let tw = HudBatch::text_width(atlas, "N");
        let (tx, ty) = (cx + nx * reach - tw / 2.0, cy + ny * reach - 4.0);
        batch.text(
            atlas,
            "N",
            tx.round(),
            ty.round(),
            rgba(255, 255, 255, 255),
            true,
        );
    }
}

/// A box of text lines (a mod's HUD panel): `(text, 0xRRGGBB)` each.
pub fn draw_lines(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    corners: &mut Corners,
    corner: Corner,
    lines: &[(&str, u32)],
) {
    if lines.is_empty() {
        return;
    }
    let w = lines
        .iter()
        .map(|(text, _)| HudBatch::text_width(atlas, text))
        .fold(0.0, f32::max)
        + 4.0;
    let h = 10.0 * lines.len() as f32 + 2.0;
    let (x, y) = corners.place(corner, w, h);
    batch.fill(atlas, x, y, w, h, BACKGROUND);
    for (i, (text, color)) in lines.iter().enumerate() {
        let [_, r, g, b] = color.to_be_bytes();
        let y = y + 2.0 + 10.0 * i as f32;
        batch.text(atlas, text, x + 2.0, y, rgba(r, g, b, 255), true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_stack_away_from_their_edge() {
        let mut c = Corners::new(400.0, 300.0, 26.0);
        assert_eq!(c.place(Corner::TopLeft, 50.0, 12.0), (2.0, 2.0));
        assert_eq!(c.place(Corner::TopLeft, 70.0, 20.0), (2.0, 16.0));
        // Top right starts under the effect icons.
        assert_eq!(c.place(Corner::TopRight, 50.0, 12.0), (348.0, 28.0));
        assert_eq!(c.place(Corner::BottomRight, 50.0, 12.0), (348.0, 286.0));
        assert_eq!(c.place(Corner::BottomRight, 50.0, 10.0), (348.0, 274.0));
        assert_
