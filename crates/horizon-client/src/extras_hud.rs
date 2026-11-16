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

pub const MAP_BTN: f32 = 30.0;
/// Xaero-style small zoom buttons at the map's bottom-right corner.
pub const MAP_ZOOM_BTN: f32 = 22.0;

/// Rectangles of the small zoom-in and zoom-out buttons at the bottom-right.
pub fn map_zoom_button_rects(width: f32, height: f32) -> ([f32; 4], [f32; 4]) {
    let bx = width - 8.0 - MAP_ZOOM_BTN;
    let zoom_out = [bx, height - 8.0 - MAP_ZOOM_BTN, MAP_ZOOM_BTN, MAP_ZOOM_BTN];
    let zoom_in = [
        bx,
        height - 8.0 - 2.0 * MAP_ZOOM_BTN - 4.0,
        MAP_ZOOM_BTN,
        MAP_ZOOM_BTN,
    ];
    (zoom_in, zoom_out)
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapMenuAction {
    CreateWaypoint,
    DeleteWaypoint,
    Close,
}

pub fn draw_big_map(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    look: &MinimapLook,
    coords: &str,
    biome: &str,
    waypoints: &[MapWaypoint],
    selected_waypoint: Option<usize>,
    center: (f64, f64),
    zoom: f32,
    cursor: (f32, f32),
    menu: Option<(f32, f32, Option<usize>)>,
) -> ([[f32; 4]; 5], Vec<([f32; 4], MapMenuAction)>) {
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
    // Xaero-inspired map chrome: a slim left rail and a compact coordinate
    // strip. Keep most of the screen available for terrain.
    batch.fill(atlas, 0.0, 0.0, 42.0, batch.height, 0xE8000000);
    batch.fill(atlas, 41.0, 0.0, 1.0, batch.height, 0x553A3A3A);
    draw_icon(
        batch,
        atlas,
        14.0,
        10.0,
        GEAR,
        2.0,
        rgba(235, 240, 245, 255),
    );
    batch.fill(atlas, 10.0, 31.0, 22.0, 1.0, 0xFF_707070);
    batch.text(
        atlas,
        &format!("{}", waypoints.len()),
        16.0,
        42.0,
        rgba(255, 220, 130, 255),
        true,
    );

    let info_w = (HudBatch::text_width(atlas, coords) + 24.0).clamp(150.0, 280.0);
    let info_x = (batch.width - info_w) * 0.5;
    batch.fill(atlas, info_x, 8.0, info_w, 32.0, 0xD9000000);
    let coords_w = HudBatch::text_width(atlas, coords);
    batch.text(
        atlas,
        coords,
        (info_x + (info_w - coords_w) * 0.5).round(),
        13.0,
        rgba(245, 248, 252, 255),
        true,
    );
    let biome_w = HudBatch::text_width(atlas, biome);
    batch.text(
        atlas,
        biome,
        (info_x + (info_w - biome_w) * 0.5).round(),
        26.0,
        rgba(205, 210, 220, 255),
        true,
    );

    // Compass rose in the upper-right corner: a cross with N/E/S/W.
    let compass_x = batch.width - 62.0;
    batch.fill(atlas, compass_x, 8.0, 52.0, 44.0, 0xE6000000);
    batch.fill(atlas, compass_x + 25.0, 20.0, 2.0, 20.0, 0xFF_9A9A9A);
    batch.fill(atlas, compass_x + 16.0, 29.0, 20.0, 2.0, 0xFF_9A9A9A);
    for (label, dx, dy, color) in [
        ("N", 23.0, 10.0, rgba(255, 255, 255, 255)),
        ("W", 7.0, 26.0, rgba(205, 210, 220, 255)),
        ("E", 40.0, 26.0, rgba(205, 210, 220, 255)),
        ("S", 23.0, 42.0, rgba(205, 210, 220, 255)),
    ] {
        batch.text(atlas, label, compass_x + dx, dy, color, true);
    }

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

    // No on-map buttons: map controls are driven by keyboard/game controls and
    // context actions (Xaero-style), keeping the terrain unobstructed.
    let zoom_label = format!("Zoom: {:.2}x", 1.0 / zoom.max(0.001));
    let zoom_w = HudBatch::text_width(atlas, &zoom_label) + 16.0;
    batch.fill(
        atlas,
        (batch.width - zoom_w) * 0.5,
        batch.height - 22.0,
        zoom_w,
        16.0,
        0xC9000000,
    );
    batch.text(
        atlas,
        &zoom_label,
        (batch.width - HudBatch::text_width(atlas, &zoom_label)) * 0.5,
        batch.height - 18.0,
        rgba(245, 248, 252, 255),
        true,
    );

    // Xaero-style virtual cursor: highlight the hovered chunk at cursor.
    let (cursor_x, cursor_y) = cursor;
    let extent = batch.width.max(batch.height);
    let per_block = extent / (crate::minimap::SIZE as f32 * zoom.max(0.001));
    let cursor_world_x = center.0
        + f64::from(cursor_x - batch.width * 0.5) * f64::from(zoom) * crate::minimap::SIZE as f64
            / f64::from(extent);
    let cursor_world_z = center.1
        + f64::from(cursor_y - batch.height * 0.5) * f64::from(zoom) * crate::minimap::SIZE as f64
            / f64::from(extent);
    let chunk_x = (cursor_world_x / 16.0).floor() * 16.0;
    let chunk_z = (cursor_world_z / 16.0).floor() * 16.0;
    let sx = batch.width * 0.5 + ((chunk_x - center.0) as f32) * per_block;
    let sy = batch.height * 0.5 + ((chunk_z - center.1) as f32) * per_block;
    let side = 16.0 * per_block;
    batch.fill(atlas, sx, sy, side, side, 0x44FFFFFF);
    batch.fill(atlas, sx, sy, side, 1.0, 0xAAFFFFFF);
    batch.fill(atlas, sx, sy + side - 1.0, side, 1.0, 0xAAFFFFFF);
    batch.fill(atlas, sx, sy, 1.0, side, 0xAAFFFFFF);
    batch.fill(atlas, sx + side - 1.0, sy, 1.0, side, 0xAAFFFFFF);
    batch.text(
        atlas,
        &format!("Cursor: X {:.0}  Z {:.0}", cursor_world_x, cursor_world_z),
        52.0,
        batch.height - 18.0,
        rgba(255, 225, 150, 255),
        true,
    );

    // Virtual mouse pointer: crosshair at the cursor (Xaero-like).
    let hx = cursor_x.round();
    let hy = cursor_y.round();
    let cursor_color = rgba(255, 255, 255, 235);
    batch.fill(atlas, hx - 5.0, hy, 3.0, 1.0, cursor_color);
    batch.fill(atlas, hx + 3.0, hy, 3.0, 1.0, cursor_color);
    batch.fill(atlas, hx, hy - 5.0, 1.0, 3.0, cursor_color);
    batch.fill(atlas, hx, hy + 3.0, 1.0, 3.0, cursor_color);

    // Small Xaero-style zoom buttons, bottom-right only.
    let (zoom_in, zoom_out) = map_zoom_button_rects(batch.width, batch.height);
    draw_map_button(batch, atlas, zoom_in, PLUS);
    draw_map_button(batch, atlas, zoom_out, MINUS);

    let mut menu_hits = Vec::new();
    if let Some((mx, my, waypoint)) = menu {
        let mut items = vec![("Create Waypoint", MapMenuAction::CreateWaypoint)];
        if waypoint.is_some() {
            items.push(("Delete Waypoint", MapMenuAction::DeleteWaypoint));
        }
        items.push(("Close", MapMenuAction::Close));
        let item_h = 16.0;
        let pad_x = 8.0;
        let pad_y = 6.0;
        let width = items
            .iter()
            .map(|(label, _)| HudBatch::text_width(atlas, label))
            .fold(0.0, f32::max)
            + pad_x * 2.0;
        let height = items.len() as f32 * item_h + pad_y * 2.0;
        let x = mx.clamp(4.0, (batch.width - width - 4.0).max(4.0));
        let y = my.clamp(4.0, (batch.height - height - 4.0).max(4.0));
        batch.fill(
            atlas,
            x - 1.0,
            y - 1.0,
            width + 2.0,
            height + 2.0,
            0xFF7A7A7A,
        );
        batch.fill(atlas, x, y, width, height, 0xF0101010);
        for (i, (label, action)) in items.iter().enumerate() {
            let iy = y + pad_y + i as f32 * item_h;
            let rect = [x, iy - 1.0, width, item_h];
            menu_hits.push((rect, *action));
            batch.text(
                atlas,
                label,
                x + pad_x,
                iy + 3.0,
                rgba(245, 248, 252, 255),
                true,
            );
        }
    }

    // Only the small zoom buttons are map controls; everything else is keys/touch.
    ([zoom_in, zoom_out, [0.0; 4], [0.0; 4], [0.0; 4]], menu_hits)
}

fn draw_map_button(batch: &mut HudBatch, atlas: &HudAtlas, r: [f32; 4], icon: &[&str]) {
    let [x, y, w, h] = r;
    // Flat, compact controls on a dark Xaero-like toolbar.
    batch.fill(atlas, x, y, w, h, 0xE6000000);
    batch.fill(atlas, x, y, w, 1.0, 0xFF_777777);
    batch.fill(atlas, x, y + h - 1.0, w, 1.0, 0xFF_777777);
    batch.fill(atlas, x, y, 1.0, h, 0xFF_777777);
    batch.fill(atlas, x + w - 1.0, y, 1.0, h, 0xFF_777777);
    let icon_w = icon.first().map_or(0, |row| row.len()) as f32 * 2.0;
    let icon_h = icon.len() as f32 * 2.0;
    draw_icon(
        batch,
        atlas,
        (x + (w - icon_w) / 2.0).round(),
        (y + (h - icon_h) / 2.0).round(),
        icon,
        2.0,
        rgba(245, 248, 252, 255),
    );
}

/// 7x7 pixel icons; `#` cells are drawn.
const GEAR: &[&str] = &[
    "#.###.#", "#######", ".##.##.", "###.###", ".##.##.", "#######", "#.###.#",
];
const PLUS: &[&str] = &[
    "...#...", "...#...", "...#...", "#######", "...#...", "...#...", "...#...",
];
const MINUS: &[&str] = &[
    ".......", ".......", ".......", "#######", ".......", ".......", ".......",
];
fn draw_icon(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    x: f32,
    y: f32,
    icon: &[&str],
    cell: f32,
    color: u32,
) {
    for (row, line) in icon.iter().enumerate() {
        for (col, c) in line.bytes().enumerate() {
            if c == b'#' {
                batch.fill(
                    atlas,
                    x + col as f32 * cell,
                    y + row as f32 * cell,
                    cell,
                    cell,
                    color,
                );
            }
        }
    }
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
