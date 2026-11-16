//! Horizon's extras in the game (`docs/extras.md`); a native mod's minimap
//! (`native_minimap`: its tiles, image and HUD) and zoom (`native_zoom`) on the camera; clicks
//! and frame times for mods (`native_clicks`, `native_frame_rate`).
//! Each hook does nothing while its extra is off and no mod uses it.
use super::{Dim, Game, Screen, native_hooks::NativeMinimap};
use horizon_client::{
    extras::{Corner, Cps, FpsCounter, fov_target},
    extras_hud::{self, Corners},
    hud::{HudBatch, MINIMAP_SPRITE},
    minimap::{self, Layer, Minimap, View},
    multiplayer,
    reconnect::{Retry, Tracker},
    title::Mode,
};
use sha2::Digest;
use std::{path::PathBuf, time::Duration};
use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Clone)]
pub(super) struct MapMenu {
    pub x: f32,
    pub y: f32,
    pub waypoint: Option<usize>,
}

#[derive(Default)]
pub(super) struct ExtrasState {
    fps: FpsCounter,
    /// Left and right mouse buttons: held, and their clicks.
    mouse: [bool; 2],
    cps: [Cps; 2],
    minimap: Minimap,
    /// The dimension the minimap's tiles belong to.
    minimap_dim: Option<Dim>,
    /// Stable per-save/per-dimension key for disk-backed explored tiles.
    minimap_cache_key: Option<String>,
    minimap_last_saved: Option<std::time::Instant>,
    /// Named map markers, saved alongside this world's explored tiles.
    map_waypoints: Vec<extras_hud::MapWaypoint>,
    selected_waypoint: Option<usize>,
    /// Auto-reconnect's attempts at the current server.
    reconnect: Tracker,
    /// Starling: the fullscreen map (the M key) is open.
    pub(super) big_map: bool,
    /// Fullscreen map zoom and pan.
    map_zoom: f32,
    map_pan: [f64; 2],
    map_drag: Option<(u64, f32, f32)>,
    /// Virtual map pointer in GUI coordinates, independent of the player.
    map_cursor: Option<[f32; 2]>,
    /// Right-click context menu on the fullscreen map.
    map_menu: Option<MapMenu>,
}
/// CPU time the minimap's tiles may take per frame.
const MINIMAP_BUDGET: Duration = Duration::from_micros(300);
/// Four map texels per world block on the 1024px fullscreen raster.
const DEFAULT_FULLSCREEN_MAP_ZOOM: f32 = 0.25;

impl Game<'_> {
    fn minimap_cache_path(&self) -> Option<PathBuf> {
        let key = self.extras.minimap_cache_key.as_ref()?;
        let root = std::env::var_os("HORIZON_PRIVATE")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HORIZON_HOME").map(PathBuf::from))
            .or_else(|| std::env::current_dir().ok())?;
        Some(root.join("map-cache").join(format!("{key}.json")))
    }

    fn save_minimap_cache(&mut self) {
        let Some(path) = self.minimap_cache_path() else {
            return;
        };
        if let Err(error) = self.extras.minimap.save_to(&path) {
            log::warn!(
                "Starling: could not save explored map cache {}: {error}",
                path.display()
            );
        } else {
            self.extras.minimap_last_saved = Some(std::time::Instant::now());
        }
    }

    fn save_map_waypoints(&self) {
        let Some(path) = self.minimap_cache_path() else {
            return;
        };
        let path = path.with_extension("waypoints.json");
        let Some(parent) = path.parent() else {
            return;
        };
        if let Err(error) = std::fs::create_dir_all(parent).and_then(|()| {
            let bytes =
                serde_json::to_vec(&self.extras.map_waypoints).map_err(std::io::Error::other)?;
            let temporary = path.with_extension("waypoints.json.tmp");
            std::fs::write(&temporary, bytes)?;
            std::fs::rename(&temporary, &path)
        }) {
            log::warn!("Starling: could not save map waypoints: {error}");
        }
    }

    fn extras_now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
    /// Mouse buttons while playing, for mods' clicks.
    pub(super) fn extras_mouse(&mut self, button: MouseButton, pressed: bool) {
        self.native_mouse_event(button, pressed);
        let i = match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            _ => return,
        };
        self.extras.mouse[i] = pressed;
        // For mods (`input.clicks-per-second`, `native_clicks`).
        if pressed {
            let now = self.extras_now();
            self.extras.cps[i].click(now);
        }
    }
    /// Once per frame: the frame's time, and keys let go behind screens.
    pub(super) fn extras_frame(&mut self, elapsed: Duration) {
        // Under agent control the window may lack focus without pausing.
        let focused = self.focused || self.control.is_some();
        let playing = self.screen == Screen::Playing && self.title.is_none() && focused;
        if !playing {
            self.extras.mouse = [false; 2];
        }
        self.update_minimap();
        // For mods (`render.frame-stats`, `native_frame_rate`).
        let now = self.extras_now();
        self.extras.fps.record(now, elapsed.as_secs_f32() * 1000.0);
        self.native_frame(elapsed);
        self.recipes_received_now();
    }
    /// The frame counter for mods: fps, min, 1% low.
    pub(super) fn frame_reading(&self) -> (u32, u32, u32) {
        let r = self.extras.fps.reading();
        (r.fps, r.min, r.low)
    }
    /// Attack and use clicks in the last second, and whether each is held.
    pub(super) fn mouse_reading(&mut self) -> ((u32, u32), [bool; 2]) {
        let now = self.extras_now();
        let [l, r] = &mut self.extras.cps;
        let cps = (l.per_second(now) as u32, r.per_second(now) as u32);
        (cps, self.extras.mouse)
    }
    /// Auto-reconnect: a connection to `address` as `name` starts.
    pub(super) fn extras_connecting(&mut self, address: &str, name: &str, lan: bool) {
        self.extras.reconnect.connecting(address, name, lan);
    }
    /// The server this game connected to last.
    pub(super) fn server_address(&self) -> Option<&str> {
        self.extras.reconnect.address()
    }
    /// Auto-reconnect: the server let the player in.
    pub(super) fn extras_joined(&mut self) {
        self.extras.reconnect.joined(std::time::Instant::now());
    }
    /// Auto-reconnect: the player left the server on purpose.
    pub(super) fn extras_left(&mut self) {
        self.extras.reconnect.reset();
    }
    /// Auto-reconnect's next attempt after a disconnect, when it is on and
    /// the reason allows one.
    pub(super) fn reconnect_after(
        &mut self,
        reason: &str,
        key: Option<&str>,
        from_server: bool,
    ) -> Option<Retry> {
        if !self.settings.extras.auto_reconnect {
            self.extras.reconnect.reset();
            return None;
        }
        let now = std::time::Instant::now();
        self.extras.reconnect.ended(reason, key, from_server, now)
    }
    /// Show the attempt's countdown on the Disconnected screen.
    pub(super) fn offer_reconnect(&mut self, retry: Option<Retry>) {
        let Some(retry) = retry else {
            return;
        };
        if let Some(title) = self.title.as_mut()
            && let Mode::Multiplayer(multiplayer::Screen::Disconnected { retry: slot, .. }) =
                &mut title.mode
        {
            log::info!(
                "Horizon: reconnecting to {} in {} s",
                horizon_client::net::shown_address(&retry.address),
                retry.seconds_left(std::time::Instant::now())
            );
            *slot = Some(retry);
        }
    }
    /// Auto-reconnect: connect again once the countdown is over. An
    /// online-mode server gets the signed-in account again: `connect_server`
    /// takes it from `auth::current()`, held in memory for the whole run
    /// (the launcher's session file is gone after the first read).
    pub(super) fn extras_retry(&mut self) -> horizon_client::Result<()> {
        if let Some(retry) = self.title.as_mut().and_then(|t| t.due_retry()) {
            self.connect_server_with_options(&retry.address, &retry.name, None, retry.lan)?;
        }
        Ok(())
    }
    /// The minimap a mod shows (`native_minimap`): its corner, its side
    /// in GUI pixels and its view of the player.
    fn minimap_shown(&self) -> Option<(Corner, f32, View)> {
        let p = self.player.as_ref()?;
        if self.title.is_some() || self.ui.hide_gui {
            return None;
        }
        let m = self.minimap_wanted()?;
        let view = View {
            // The terrain view pans independently; the player marker is offset
            // in the HUD by the inverse of this displacement.
            x: p.position.x
                + if self.extras.big_map {
                    self.extras.map_pan[0]
                } else {
                    0.0
                },
            z: p.position.z
                + if self.extras.big_map {
                    self.extras.map_pan[1]
                } else {
                    0.0
                },
            yaw: self.yaw,
            rotate: m.rotate,
            blocks_per_pixel: m.blocks_per_pixel,
            round: m.round,
        };
        Some((m.corner, m.size, view))
    }
    /// The minimap shown: a native mod's (`native_minimap`).
    fn minimap_wanted(&self) -> Option<NativeMinimap> {
        // Starling: a mod's minimap wins; with none installed, show a built-in
        // default so no native-mod compile is needed.
        let map = self.native_minimap_now().unwrap_or(NativeMinimap {
            corner: Corner::TopRight,
            size: 72.0,
            blocks_per_pixel: 1.0,
            rotate: false,
            round: false,
        });
        // The fullscreen map (M) shows a wider area, north up.
        Some(if self.extras.big_map {
            NativeMinimap {
                blocks_per_pixel: if self.extras.map_zoom > 0.0 {
                    self.extras.map_zoom
                } else {
                    DEFAULT_FULLSCREEN_MAP_ZOOM
                },
                rotate: false,
                round: false,
                ..map
            }
        } else {
            map
        })
    }
    // The Android-only touch HUD calls this; host CI builds without that module enabled.
    #[allow(dead_code)]
    /// Starling: M opens and closes the fullscreen map, and Escape closes it
    /// before it can open the pause menu. True when the key was used.
    pub(super) fn map_key_press(&mut self, key: KeyCode) -> bool {
        if self.screen != Screen::Playing || self.title.is_some() {
            return false;
        }
        if self.extras.big_map {
            let [width, height] = self.touch_size();
            let scale = HudBatch::new([width, height]).scale;
            let bounds = [width as f32 / scale, height as f32 / scale];
            let cursor = self
                .extras
                .map_cursor
                .unwrap_or([bounds[0] * 0.5, bounds[1] * 0.5]);
            let step = 18.0;
            match key {
                KeyCode::ArrowLeft | KeyCode::KeyA => {
                    self.extras.map_menu = None;
                    self.extras.map_cursor =
                        Some([(cursor[0] - step).clamp(0.0, bounds[0]), cursor[1]]);
                    return true;
                }
                KeyCode::ArrowRight | KeyCode::KeyD => {
                    self.extras.map_menu = None;
                    self.extras.map_cursor =
                        Some([(cursor[0] + step).clamp(0.0, bounds[0]), cursor[1]]);
                    return true;
                }
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.extras.map_menu = None;
                    self.extras.map_cursor =
                        Some([cursor[0], (cursor[1] - step).clamp(0.0, bounds[1])]);
                    return true;
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.extras.map_menu = None;
                    self.extras.map_cursor =
                        Some([cursor[0], (cursor[1] + step).clamp(0.0, bounds[1])]);
                    return true;
                }
                KeyCode::Equal | KeyCode::NumpadAdd => {
                    self.extras.map_menu = None;
                    let z = if self.extras.map_zoom > 0.0 {
                        self.extras.map_zoom
                    } else {
                        DEFAULT_FULLSCREEN_MAP_ZOOM
                    };
                    self.extras.map_zoom = (z * 0.8).clamp(0.125, 64.0);
                    return true;
                }
                KeyCode::Minus | KeyCode::NumpadSubtract => {
                    self.extras.map_menu = None;
                    let z = if self.extras.map_zoom > 0.0 {
                        self.extras.map_zoom
                    } else {
                        DEFAULT_FULLSCREEN_MAP_ZOOM
                    };
                    self.extras.map_zoom = (z / 0.8).clamp(0.125, 64.0);
                    return true;
                }
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    self.extras.map_menu = None;
                    self.extras.map_cursor = Some(cursor);
                    self.map_add_waypoint_at_cursor(cursor, bounds);
                    return true;
                }
                KeyCode::Backspace | KeyCode::Delete => {
                    self.extras.map_menu = None;
                    self.map_delete_selected_waypoint();
                    return true;
                }
                KeyCode::Escape => {
                    if self.extras.map_menu.take().is_some() {
                        return true;
                    }
                    self.extras.big_map = false;
                    self.extras.map_drag = None;
                    return true;
                }
                _ => {}
            }
        }
        match key {
            KeyCode::KeyM => {
                self.extras.big_map = !self.extras.big_map;
                self.extras.map_cursor = None;
                self.extras.map_drag = None;
                self.extras.map_menu = None;
                true
            }
            KeyCode::Escape if self.extras.big_map => {
                self.extras.map_menu = None;
                self.extras.big_map = false;
                self.extras.map_drag = None;
                true
            }
            _ => false,
        }
    }
    /// Capture touches while the fullscreen map is open so they do not move the
    /// player. Tapping positions the cursor (and selects a waypoint if hit);
    /// dragging pans the map. Map actions use game controls/keys, not on-map buttons.
    pub(super) fn fullscreen_map_touch(
        &mut self,
        id: u64,
        phase: winit::event::TouchPhase,
        px: f32,
        py: f32,
    ) -> bool {
        if !self.extras.big_map || self.screen != Screen::Playing {
            return false;
        }
        let [w, h] = self.touch_size();
        let scale = HudBatch::new([w, h]).scale;
        let (x, y) = (px / scale, py / scale);
        match phase {
            winit::event::TouchPhase::Started => {
                self.extras.map_cursor = Some([x, y]);
                // Small Xaero-style zoom buttons (bottom-right).
                let bounds = [w as f32 / scale, h as f32 / scale];
                let (zoom_in, zoom_out) = extras_hud::map_zoom_button_rects(bounds[0], bounds[1]);
                let inside =
                    |r: [f32; 4]| x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3];
                if inside(zoom_in) || inside(zoom_out) {
                    self.extras.map_menu = None;
                    let z = if self.extras.map_zoom > 0.0 {
                        self.extras.map_zoom
                    } else {
                        DEFAULT_FULLSCREEN_MAP_ZOOM
                    };
                    self.extras.map_zoom = if inside(zoom_in) {
                        (z * 0.8).clamp(0.125, 64.0)
                    } else {
                        (z / 0.8).clamp(0.125, 64.0)
                    };
                    self.extras.map_drag = None;
                    return true;
                }
                if self.extras.map_menu.is_some() && self.map_handle_gui_click(x, y) {
                    self.extras.map_drag = None;
                    return true;
                }
                let zoom = if self.extras.map_zoom > 0.0 {
                    self.extras.map_zoom
                } else {
                    DEFAULT_FULLSCREEN_MAP_ZOOM
                };
                let extent = f64::from(w.max(h) as f32 / scale);
                let center = self.player.as_ref().map(|player| {
                    (
                        player.position.x + self.extras.map_pan[0],
                        player.position.z + self.extras.map_pan[1],
                    )
                });
                let selected = center.and_then(|center| {
                    self.extras.map_waypoints.iter().enumerate().rev().find_map(
                        |(index, waypoint)| {
                            let wx = f64::from(w as f32 / scale * 0.5)
                                + (waypoint.x - center.0) / f64::from(zoom) * extent
                                    / minimap::SIZE as f64;
                            let wy = f64::from(h as f32 / scale * 0.5)
                                + (waypoint.z - center.1) / f64::from(zoom) * extent
                                    / minimap::SIZE as f64;
                            ((wx - f64::from(x)).abs() <= 14.0 && (wy - f64::from(y)).abs() <= 14.0)
                                .then_some(index)
                        },
                    )
                });
                self.extras.selected_waypoint = selected;
                self.extras.map_drag = if selected.is_some() {
                    None
                } else {
                    Some((id, x, y))
                };
                true
            }
            winit::event::TouchPhase::Moved => {
                if let Some((finger, ox, oy)) = self.extras.map_drag
                    && finger == id
                {
                    let z = if self.extras.map_zoom > 0.0 {
                        self.extras.map_zoom
                    } else {
                        DEFAULT_FULLSCREEN_MAP_ZOOM
                    };
                    // The square source texture is cover-scaled to the viewport.
                    // Convert screen drag distance back to world blocks.
                    let viewport = f64::from(w.max(h) as f32 / scale);
                    let blocks_per_gui_pixel = f64::from(z) * minimap::SIZE as f64 / viewport;
                    self.extras.map_pan[0] -= f64::from(x - ox) * blocks_per_gui_pixel;
                    self.extras.map_pan[1] -= f64::from(y - oy) * blocks_per_gui_pixel;
                    self.extras.map_drag = Some((id, x, y));
                }
                true
            }
            winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                if self
                    .extras
                    .map_drag
                    .is_some_and(|(finger, _, _)| finger == id)
                {
                    self.extras.map_drag = None;
                }
                true
            }
        }
    }
    /// Refresh the minimap's tiles within the frame budget, recompose its
    /// image when the view or a tile changed, and queue the upload.
    fn update_minimap(&mut self) {
        let Some((_, _, view)) = self.minimap_shown() else {
            // Flush explored tiles when leaving a world, so closing the app
            // immediately afterwards does not discard the latest discoveries.
            self.save_minimap_cache();
            if self.minimap_wanted().is_none() {
                self.extras.minimap.clear();
                self.extras.minimap_dim = None;
                self.extras.minimap_cache_key = None;
            }
            return;
        };
        // world_root distinguishes single-player saves and remote server save roots;
        // dimension is part of the identity so Nether/Overworld maps never mix.
        let identity = format!(
            "{}::{}::{:?}",
            self.world_root.display(),
            self.server_address().unwrap_or("singleplayer"),
            self.dim
        );
        let cache_key = format!("{:x}", sha2::Sha256::digest(identity.as_bytes()));
        if self.extras.minimap_cache_key.as_deref() != Some(cache_key.as_str()) {
            self.save_minimap_cache();
            self.extras.minimap.clear();
            self.extras.minimap_dim = Some(self.dim);
            self.extras.minimap_cache_key = Some(cache_key);
            self.extras.minimap_last_saved = None;
            if let Some(path) = self.minimap_cache_path() {
                match self.extras.minimap.load_from(&path) {
                    Ok(true) => log::info!(
                        "Starling: restored {} explored map chunks",
                        self.extras.minimap.tile_count()
                    ),
                    Ok(false) => {}
                    Err(error) => log::warn!(
                        "Starling: could not load explored map cache {}: {error}",
                        path.display()
                    ),
                }
                let waypoints_path = path.with_extension("waypoints.json");
                self.extras.selected_waypoint = None;
                self.extras.map_waypoints = match std::fs::read(&waypoints_path) {
                    Ok(bytes) => serde_json::from_slice::<Vec<extras_hud::MapWaypoint>>(&bytes)
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|w| w.name.len() <= 48 && w.x.is_finite() && w.z.is_finite())
                        .take(128)
                        .collect(),
                    Err(_) => Vec::new(),
                };
            }
        } else if self.extras.minimap_dim != Some(self.dim) {
            self.save_minimap_cache();
            self.extras.minimap.clear();
            self.extras.minimap_dim = Some(self.dim);
        }
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let head = (p.position.y + f64::from(p.dimensions.eye_height)).floor() as i32;
        let (bx, bz) = (view.x.floor() as i32, view.z.floor() as i32);
        // Underground, under a roof or in the Nether: the floor around the player.
        let covered = self.world.light(bx, head, bz) >> 4 == 0;
        let layer = if self.dim == Dim::Nether || covered {
            Layer::Below(head)
        } else {
            Layer::Surface
        };
        // The chunks the image can reach (its corners when it turns).
        let reach = minimap::SIZE as f32 / 2.0 * view.blocks_per_pixel * std::f32::consts::SQRT_2;
        let radius = ((reach / 16.0).ceil() as i32 + 1).min(self.settings.render_distance.max(2));
        self.extras.minimap.update(
            &self.world,
            self.res.facts(),
            (bx >> 4, bz >> 4),
            radius,
            layer,
            MINIMAP_BUDGET,
        );
        // Persist at a low frequency while exploring; the title/disconnect path
        // above performs an immediate final flush.
        if self
            .extras
            .minimap_last_saved
            .is_none_or(|last| last.elapsed() >= Duration::from_secs(3))
        {
            self.save_minimap_cache();
        }
        if !self.extras.minimap.compose(view) {
            return;
        }
        let Some(rect) = self.gui.atlas.sprites.get(MINIMAP_SPRITE).copied() else {
            return;
        };
        let (w, h) = (self.gui.atlas.width as f32, self.gui.atlas.height as f32);
        self.map_patches.push(horizon_client::font::Patch {
            x: (rect.u0 * w).round() as u32,
            y: (rect.v0 * h).round() as u32,
            width: minimap::SIZE as u32,
            height: minimap::SIZE as u32,
            rgba: self.extras.minimap.image.clone(),
        });
    }
    /// A mod's zoom in effect (`native_zoom`): (factor, smooth, turning
    /// scale); none is 1.
    fn zoom_now(&self) -> (f32, bool, f64) {
        self.native_zoom_now().map_or((1.0, true, 1.0), |z| {
            (z.factor, z.smooth, f64::from(z.turn))
        })
    }
    /// The field-of-view modifier target, divided by a smooth zoom.
    pub(super) fn zoomed_fov_target(&self, vanilla: f32) -> f32 {
        match self.zoom_now() {
            (factor, true, _) => fov_target(vanilla, factor),
            _ => vanilla,
        }
    }
    /// The field of view in degrees, divided by an instant zoom.
    pub(super) fn zoomed_fov_degrees(&self, fov: f32) -> f32 {
        match self.zoom_now() {
            (factor, false, _) => fov / factor,
            _ => fov,
        }
    }

    /// Mouse wheel while the fullscreen map is open zooms the map (Xaero-like).
    pub(super) fn map_mouse_wheel(&mut self, delta: f32) -> bool {
        if !self.extras.big_map || self.screen != Screen::Playing || self.title.is_some() {
            return false;
        }
        if delta == 0.0 {
            return true;
        }
        self.extras.map_menu = None;
        let z = if self.extras.map_zoom > 0.0 {
            self.extras.map_zoom
        } else {
            DEFAULT_FULLSCREEN_MAP_ZOOM
        };
        let factor = if delta > 0.0 {
            0.9f32.powf(delta.abs().min(10.0))
        } else {
            1.1f32.powf(delta.abs().min(10.0))
        };
        self.extras.map_zoom = (z * factor).clamp(0.125, 64.0);
        true
    }

    fn map_menu_layout(
        &self,
        x: f32,
        y: f32,
        bounds: [f32; 2],
        waypoint: Option<usize>,
    ) -> Vec<([f32; 4], extras_hud::MapMenuAction)> {
        let mut items = vec![("Create Waypoint", extras_hud::MapMenuAction::CreateWaypoint)];
        if waypoint.is_some() {
            items.push(("Delete Waypoint", extras_hud::MapMenuAction::DeleteWaypoint));
        }
        items.push(("Close", extras_hud::MapMenuAction::Close));
        let item_h = 16.0;
        let pad_x = 8.0;
        let pad_y = 6.0;
        let width = items
            .iter()
            .map(|(label, _)| {
                // Approximate text width to match HUD (8px per glyph is close enough).
                label.chars().count() as f32 * 6.0
            })
            .fold(0.0, f32::max)
            + pad_x * 2.0;
        let height = items.len() as f32 * item_h + pad_y * 2.0;
        let mx = x.clamp(4.0, (bounds[0] - width - 4.0).max(4.0));
        let my = y.clamp(4.0, (bounds[1] - height - 4.0).max(4.0));
        items
            .into_iter()
            .enumerate()
            .map(|(i, (_, action))| {
                let iy = my + pad_y + i as f32 * item_h;
                ([mx, iy - 1.0, width, item_h], action)
            })
            .collect()
    }

    fn map_add_waypoint_at_cursor(&mut self, cursor: [f32; 2], bounds: [f32; 2]) {
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let zoom = if self.extras.map_zoom > 0.0 {
            self.extras.map_zoom
        } else {
            DEFAULT_FULLSCREEN_MAP_ZOOM
        };
        let extent = f64::from(bounds[0].max(bounds[1]));
        let center_x = player.position.x + self.extras.map_pan[0];
        let center_z = player.position.z + self.extras.map_pan[1];
        let world_x = center_x
            + f64::from(cursor[0] - bounds[0] * 0.5) * f64::from(zoom) * minimap::SIZE as f64
                / extent;
        let world_z = center_z
            + f64::from(cursor[1] - bounds[1] * 0.5) * f64::from(zoom) * minimap::SIZE as f64
                / extent;
        let number = self.extras.map_waypoints.len() + 1;
        self.extras.map_waypoints.push(extras_hud::MapWaypoint {
            name: format!("Waypoint {number}"),
            x: world_x,
            z: world_z,
        });
        self.extras.selected_waypoint = Some(self.extras.map_waypoints.len() - 1);
        self.save_map_waypoints();
    }

    fn map_delete_selected_waypoint(&mut self) {
        let remove = self
            .extras
            .selected_waypoint
            .filter(|&index| index < self.extras.map_waypoints.len())
            .or_else(|| self.extras.map_waypoints.len().checked_sub(1));
        if let Some(index) = remove {
            self.extras.map_waypoints.remove(index);
        }
        self.extras.selected_waypoint = None;
        self.save_map_waypoints();
    }

    pub(super) fn map_open_context_menu(&mut self) -> bool {
        if !self.extras.big_map || self.screen != Screen::Playing || self.title.is_some() {
            return false;
        }
        let [w, h] = self.touch_size();
        let scale = HudBatch::new([w, h]).scale;
        let bounds = [w as f32 / scale, h as f32 / scale];
        let cursor = self
            .extras
            .map_cursor
            .unwrap_or([bounds[0] * 0.5, bounds[1] * 0.5]);
        self.extras.map_cursor = Some(cursor);
        self.extras.map_menu = Some(MapMenu {
            x: cursor[0],
            y: cursor[1],
            waypoint: self.extras.selected_waypoint,
        });
        true
    }

    pub(super) fn map_handle_gui_click(&mut self, x: f32, y: f32) -> bool {
        let Some(menu) = self.extras.map_menu.clone() else {
            return false;
        };
        let [w, h] = self.touch_size();
        let scale = HudBatch::new([w, h]).scale;
        let bounds = [w as f32 / scale, h as f32 / scale];
        for (rect, action) in self.map_menu_layout(menu.x, menu.y, bounds, menu.waypoint) {
            let [rx, ry, rw, rh] = rect;
            if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
                match action {
                    extras_hud::MapMenuAction::CreateWaypoint => {
                        let cursor = self
                            .extras
                            .map_cursor
                            .unwrap_or([bounds[0] * 0.5, bounds[1] * 0.5]);
                        self.map_add_waypoint_at_cursor(cursor, bounds);
                    }
                    extras_hud::MapMenuAction::DeleteWaypoint => {
                        self.extras.selected_waypoint =
                            menu.waypoint.or(self.extras.selected_waypoint);
                        self.map_delete_selected_waypoint();
                    }
                    extras_hud::MapMenuAction::Close => {}
                }
                self.extras.map_menu = None;
                return true;
            }
        }
        // Click outside menu dismisses it.
        self.extras.map_menu = None;
        false
    }

    pub(super) fn map_mouse_click(&mut self, button: MouseButton, pressed: bool) -> bool {
        if !self.extras.big_map || self.screen != Screen::Playing || self.title.is_some() {
            return false;
        }
        if !pressed {
            return self.extras.map_menu.is_some();
        }
        let [w, h] = self.touch_size();
        let scale = HudBatch::new([w, h]).scale;
        let x = self.cursor.0 / scale;
        let y = self.cursor.1 / scale;
        if self.extras.map_menu.is_some() {
            self.map_handle_gui_click(x, y);
            return true;
        }
        // Left click on map (when menu closed): update cursor/selection similar to touch tap.
        if button == MouseButton::Left {
            self.extras.map_cursor = Some([x, y]);
            let bounds = [w as f32 / scale, h as f32 / scale];
            let (zoom_in, zoom_out) = extras_hud::map_zoom_button_rects(bounds[0], bounds[1]);
            let inside =
                |r: [f32; 4]| x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3];
            if inside(zoom_in) || inside(zoom_out) {
                let z = if self.extras.map_zoom > 0.0 {
                    self.extras.map_zoom
                } else {
                    DEFAULT_FULLSCREEN_MAP_ZOOM
                };
                self.extras.map_zoom = if inside(zoom_in) {
                    (z * 0.8).clamp(0.125, 64.0)
                } else {
                    (z / 0.8).clamp(0.125, 64.0)
                };
                return true;
            }
            let zoom = if self.extras.map_zoom > 0.0 {
                self.extras.map_zoom
            } else {
                DEFAULT_FULLSCREEN_MAP_ZOOM
            };
            let extent =
                f64::from(w as f32 / scale.max(1.0)).max(f64::from(h as f32 / scale.max(1.0)));
            let center = self.player.as_ref().map(|player| {
                (
                    player.position.x + self.extras.map_pan[0],
                    player.position.z + self.extras.map_pan[1],
                )
            });
            let selected =
                center.and_then(|center| {
                    self.extras.map_waypoints.iter().enumerate().rev().find_map(
                        |(index, waypoint)| {
                            let wx = f64::from(w as f32 / scale * 0.5)
                                + (waypoint.x - center.0) / f64::from(zoom) * extent
                                    / minimap::SIZE as f64;
                            let wy = f64::from(h as f32 / scale * 0.5)
                                + (waypoint.z - center.1) / f64::from(zoom) * extent
                                    / minimap::SIZE as f64;
                            ((wx - f64::from(x)).abs() <= 14.0 && (wy - f64::from(y)).abs() <= 14.0)
                                .then_some(index)
                        },
                    )
                });
            self.extras.selected_waypoint = selected;
            // Don't start drag from mouse click here; wheel/keys handle most desktop use.
            return true;
        }
        false
    }
    /// Mouse turning while zoomed.
    pub(super) fn zoom_turn_scale(&self) -> f64 {
        self.zoom_now().2
    }
    /// The minimap and mods' corner panels, in the HUD layer (hidden with F1
    /// by the caller). The top left corner gives way to the debug screen.
    pub(super) fn draw_extras(&mut self, batch: &mut HudBatch) {
        let map = self.minimap_shown();
        let big = self.extras.big_map;
        let visible = |corner| big || !(self.debug && corner == Corner::TopLeft);
        let mut corners = Corners::new(batch.width, batch.height, self.effect_rows_height());
        let atlas = &self.gui.atlas;
        if let Some((map_corner, map_size, view)) = map
            && visible(map_corner)
            && !self.extras.minimap.image.is_empty()
            && let Some(image) = atlas.sprites.get(MINIMAP_SPRITE).copied()
        {
            let (arrow, north) = if view.rotate {
                (0.0, Some(view.north()))
            } else {
                (180.0 + view.yaw, None)
            };
            let map_zoom = if self.extras.map_zoom > 0.0 {
                self.extras.map_zoom
            } else {
                DEFAULT_FULLSCREEN_MAP_ZOOM
            };
            let map_extent = batch.height.max(batch.width);
            let player_offset = if big {
                (
                    (-self.extras.map_pan[0] / f64::from(map_zoom) * f64::from(map_extent)
                        / minimap::SIZE as f64) as f32,
                    (-self.extras.map_pan[1] / f64::from(map_zoom) * f64::from(map_extent)
                        / minimap::SIZE as f64) as f32,
                )
            } else {
                (0.0, 0.0)
            };
            let look = extras_hud::MinimapLook {
                corner: map_corner,
                size: if big { map_extent } else { map_size },
                image,
                arrow,
                north,
                player_offset,
            };
            if big {
                let (px, pz) = self
                    .player
                    .as_ref()
                    .map_or((view.x, view.z), |p| (p.position.x, p.position.z));
                let py = self.player.as_ref().map_or(0.0, |p| p.position.y);
                let coords = format!("X: {}  Y: {}  Z: {}", px.floor(), py.floor(), pz.floor());
                let biome = self.map_biome_name();
                let map_cursor = self
                    .extras
                    .map_cursor
                    .unwrap_or([batch.width * 0.5, batch.height * 0.5]);
                let menu = self
                    .extras
                    .map_menu
                    .as_ref()
                    .map(|m| (m.x, m.y, m.waypoint));
                let _ = extras_hud::draw_big_map(
                    batch,
                    atlas,
                    &look,
                    &coords,
                    &biome,
                    &self.extras.map_waypoints,
                    self.extras.selected_waypoint,
                    (view.x, view.z),
                    map_zoom,
                    (map_cursor[0], map_cursor[1]),
                    menu,
                );
            } else {
                extras_hud::draw_minimap(batch, atlas, &mut corners, &look);
            }
        }
        if !big {
            self.native_hud(batch, &mut corners);
        }
    }
    /// The biome at the player's feet, as "Plains" for the map screen.
    fn map_biome_name(&self) -> String {
        let Some(p) = self.player.as_ref() else {
            return String::new();
        };
        let (x, y, z) = (
            p.position.x.floor() as i32,
            p.position.y.floor() as i32,
            p.position.z.floor() as i32,
        );
        if y < self.world.bottom_y() || y >= self.world.top_y() {
            return String::new();
        }
        let Some(chunk) = self.world.chunks.get(&(x >> 4, z >> 4)) else {
            return String::new();
        };
        let Some(biome) = self.res.biomes().list.get(chunk.biome(x, y, z) as usize) else {
            return String::new();
        };
        let name = biome.name.to_string();
        let name = name.rsplit(':').next().unwrap_or("");
        name.split('_')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    /// The height the status effect icons take at the top right
    /// (`Hud.extractEffects`: beneficial at y 1, harmful at y 26).
    fn effect_rows_height(&self) -> f32 {
        let effects = &self.potions.effects.list;
        if effects.is_empty() || self.screen != Screen::Playing {
            return 0.0;
        }
        let harmful = effects.iter().any(|e| {
            horizon_client::brewing::effect(e.id)
                .is_some_and(|d| d.category != horizon_client::brewing::Category::Beneficial)
        });
        if harmful { 50.0 } else { 25.0 }
    }
}
