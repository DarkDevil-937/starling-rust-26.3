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
    used: [f32; 4],
}
impl Corners {
    pub fn new(width: f32, height: f32, top_right: f32) -> Self {
        Self {
            width,
            height,
            used: [0.0, top_right, 0.0, 0.0],
        }
    }
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

pub struct MinimapLook {
    pub corner: Corner,
    pub size: f32,
    pub image: Rect,
    pub arrow: f32,
    pub north: Option<(f32, f32)>,
    /// Screen-space displacement of the real player marker from map centre.
    pub player_offset: (f32, f32),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MapWaypoint {
    pub name: String,
    pub x: f64,
    pub z: f64,
}

pub const MAP_BTN: f32 = 36.0;

pub fn draw_minimap(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    corners: &mut Corners,
    look: &MinimapLook,
) {
    let (x, y) = corners.place(look.corner, look.size, look.size);
    // Framed, pixel-style minimap panel inspired by classic Minecraft map mods.
    batch.fill(
        atlas,
        x - 3.0,
        y - 3.0,
        look.size + 6.0,
        look.size + 6.0,
        0xFF_101010,
    );
    batch.fill(
        atlas,
        x - 2.0,
        y - 2.0,
        look.size + 4.0,
        look.size + 4.0,
        0xFF_C8C8C8,
    );
    batch.fill(
        atlas,
        x - 1.0,
        y - 1.0,
        look.size + 2.0,
        look.size + 2.0,
        0xFF_303030,
    );
    draw_map_at(batch, atlas, look, (x, y), 4.0);
    if look.north.is_none() {
        let (cx, cy) = (x + look.size / 2.0, y + look.size / 2.0);
        let color = rgba(255, 255, 255, 230);
        for (label, tx, ty) in [
            ("N", cx - 2.0, y + 2.0),
            ("E", x + look.size - 8.0, cy - 4.0),
            ("S", cx - 2.0, y + look.size - 9.0),
            ("W", x + 2.0, cy - 4.0),
        ] {
            batch.text(atlas, label, tx.round(), ty.round(), color, true);
        }
    }
}

pub fn draw_big_map(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    look: &MinimapLook,
    coords: &str,
    waypoints: &[MapWaypoint],
    selected_waypoint: Option<usize>,
    center: (f64, f64),
    zoom: f32,
    cursor: (f32, f32),
) -> [[f32; 4]; 5] {
    batch.fill(atlas, 0.0, 0.0, batch.width, batch.height, BACKGROUND);
    // Fill the landscape viewport without distorting the square terrain
    // texture: crop the vertical edges (or horizontal edges on portrait).
    let mut image = look.image;
    if batch.width > batch.height {
        let visible = batch.height / batch.width;
        let crop = (1.0 - visible) * 0.5;
        let span = image.v1 - image.v0;
        image.v0 += span * crop;
        image.v1 -= span * crop;
    } else if batch.height > batch.width {
        let visible = batch.width / batch.height;
        let crop = (1.0 - visible) * 0.5;
        let span = image.u1 - image.u0;
        image.u0 += span * crop;
        image.u1 -= span * crop;
    }
    batch.rect_uv(0.0, 0.0, batch.width, batch.height, image, u32::MAX);
    let (cx, cy) = (
        batch.width / 2.0 + look.player_offset.0,
        batch.height / 2.0 + look.player_offset.1,
    );
    // Keep the player marker tied to the real player position while the map
    // is dragged underneath it. Hide it when the player is outside the view.
    if cx >= 12.0
        && cy >= 12.0
        && cx <= batch.width - 12.0
        && cy <= batch.height - 12.0
        && let Some(r) = atlas.sprites.get("map/decorations/player").copied()
    {
        let (s, c) = (look.arrow + 180.0).to_radians().sin_cos();
        let half = 9.0;
        let p = |dx: f32, dy: f32| [cx + dx * c - dy * s, cy + dx * s + dy * c];
        batch.quad(
            [
                p(-half, half),
                p(half, half),
                p(half, -half),
                p(-half, -half),
            ],
            [[r.u0, r.v0], [r.u1, r.v0], [r.u1, r.v1], [r.u0, r.v1]],
            u32::MAX,
        );
    }
    // Compact information card on the left; terrain remains edge-to-edge.
    batch.fill(atlas, 10.0, 10.0, 190.0, 52.0, 0xE0161B22);
    batch.fill(atlas, 10.0, 10.0, 2.0, 52.0, 0xFF_65B8E8);
    batch.text(atlas, "WORLD", 20.0, 17.0, rgba(130, 205, 255, 255), true);
    batch.text(atlas, coords, 20.0, 31.0, rgba(245, 248, 252, 255), true);
    batch.text(
        atlas,
        &format!("{} WAYPOINTS", waypoints.len()),
        20.0,
        45.0,
        rgba(190, 205, 220, 255),
        true,
    );

    // Waypoint positions use the same world-to-screen transform as the terrain.
    let extent = batch.height.max(batch.width);
    for (index, waypoint) in waypoints.iter().enumerate() {
        let x = batch.width * 0.5
            + ((waypoint.x - center.0) / f64::from(zoom) * f64::from(extent)
                / crate::minimap::SIZE as f64) as f32;
        let y = batch.height * 0.5
            + ((waypoint.z - center.1) / f64::from(zoom) * f64::from(extent)
                / crate::minimap::SIZE as f64) as f32;
        if x >= 10.0 && y >= 10.0 && x <= batch.width - 10.0 && y <= batch.height - 10.0 {
            let marker = if selected_waypoint == Some(index) {
                0xFF70D6FF
            } else {
                0xFFFFC857
            };
            batch.fill(atlas, x - 5.0, y - 5.0, 10.0, 10.0, marker);
            batch.fill(atlas, x - 1.0, y - 1.0, 2.0, 2.0, 0xFF202020);
            batch.text(
                atlas,
                &waypoint.name,
                x + 8.0,
                y - 4.0,
                rgba(255, 230, 160, 255),
                true,
            );
        }
    }

    let pad = 8.0;
    let bx = batch.width - pad - MAP_BTN;
    let close = [bx, pad, MAP_BTN, MAP_BTN];
    let zoom_in = [bx, pad + MAP_BTN + 6.0, MAP_BTN, MAP_BTN];
    let zoom_out = [bx, pad + 2.0 * (MAP_BTN + 6.0), MAP_BTN, MAP_BTN];
    let add_waypoint = [bx, pad + 3.0 * (MAP_BTN + 6.0), MAP_BTN, MAP_BTN];
    let remove_waypoint = [bx, pad + 4.0 * (MAP_BTN + 6.0), MAP_BTN, MAP_BTN];

    draw_map_button(batch, atlas, close, "X");
    draw_map_button(batch, atlas, zoom_in, "+");
    draw_map_button(batch, atlas, zoom_out, "-");
    draw_map_button(batch, atlas, add_waypoint, "W");
    draw_map_button(batch, atlas, remove_waypoint, "D");

    // Xaero-style virtual cursor; arrow keys move it and Enter activates controls.
    let (cursor_x, cursor_y) = cursor;
    batch.fill(
        atlas,
        cursor_x - 10.0,
        cursor_y - 1.0,
        20.0,
        2.0,
        0xFF_101820,
    );
    batch.fill(
        atlas,
        cursor_x - 1.0,
        cursor_y - 10.0,
        2.0,
        20.0,
        0xFF_101820,
    );
    batch.fill(atlas, cursor_x - 8.0, cursor_y - 1.0, 16.0, 1.0, 0xFFFFE17A);
    batch.fill(atlas, cursor_x - 1.0, cursor_y - 8.0, 1.0, 16.0, 0xFFFFE17A);
    let extent = batch.width.max(batch.height);
    let cursor_world_x = center.0
        + f64::from(cursor_x - batch.width * 0.5) * f64::from(zoom) * crate::minimap::SIZE as f64
            / f64::from(extent);
    let cursor_world_z = center.1
        + f64::from(cursor_y - batch.height * 0.5) * f64::from(zoom) * crate::minimap::SIZE as f64
            / f64::from(extent);
    batch.text(
        atlas,
        &format!("X {:.1}  Z {:.1}", cursor_world_x, cursor_world_z),
        14.0,
        batch.height - 18.0,
        rgba(255, 225, 150, 255),
        true,
    );

    [close, zoom_in, zoom_out, add_waypoint, remove_waypoint]
}

fn draw_map_button(batch: &mut HudBatch, atlas: &HudAtlas, r: [f32; 4], label: &str) {
    let [x, y, w, h] = r;
    // Flat translucent controls with a fine outline; no chunky bevels.
    batch.fill(atlas, x, y, w, h, 0xD91A222C);
    batch.fill(atlas, x, y, w, 1.0, 0xFF_8A9AAA);
    batch.fill(atlas, x, y + h - 1.0, w, 1.0, 0xFF_8A9AAA);
    batch.fill(atlas, x, y, 1.0, h, 0xFF_8A9AAA);
    batch.fill(atlas, x + w - 1.0, y, 1.0, h, 0xFF_8A9AAA);
    let tw = HudBatch::text_width(atlas, label);
    let tx = x + (w - tw) / 2.0;
    let ty = y + (h - 8.0) / 2.0 - 1.0;
    batch.text(
        atlas,
        label,
        tx.round(),
        ty.round(),
        rgba(245, 248, 252, 255),
        true,
    );
}

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
        assert_eq!(c.place(Corner::TopRight, 50.0, 12.0), (348.0, 28.0));
        assert_eq!(c.place(Corner::BottomRight, 50.0, 12.0), (348.0, 286.0));
        assert_eq!(c.place(Corner::BottomRight, 50.0, 10.0), (348.0, 274.0));
        assert_eq!(c.place(Corner::BottomLeft, 10.0, 10.0), (2.0, 288.0));
    }
}
