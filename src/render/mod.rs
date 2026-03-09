pub mod ui;
pub mod util;

use self::{ui::*, util::*};

use crate::{
    game::{CursorState, GameUI, InputState},
    model::*,
    prelude::*,
    ui::layout::AreaOps,
};

/// Full size of a single tile in pixels, used for scaling textures to properly fit on the tile.
const TILE_SIZE_PIXELS: vec2<usize> = vec2(32, 32);

pub struct GameRender {
    pub context: Context,
    pub util: UtilRender,
    pub ui: UiRender,
    pub active_highlight: (vec2<ICoord>, Time),
}

impl GameRender {
    pub fn new(context: Context) -> Self {
        Self {
            util: UtilRender::new(context.clone()),
            ui: UiRender::new(context.clone()),
            active_highlight: (vec2::ZERO, Time::ZERO),
            context,
        }
    }

    pub fn draw_game(
        &mut self,
        model: &Model,
        cursor: &CursorState,
        input_state: &InputState,
        framebuffer: &mut ugli::Framebuffer,
        delta_time: Time,
    ) {
        let assets = &self.context.assets;
        let sprites = &assets.sprites;
        let palette = &assets.palette;

        if Some(self.active_highlight.0) != cursor.grid_pos {
            self.active_highlight.1 -= delta_time / r32(0.25);
            if self.active_highlight.1 <= Time::ZERO
                && let Some(grid_pos) = cursor.grid_pos
            {
                self.active_highlight.0 = grid_pos;
            }
        } else {
            self.active_highlight.1 += delta_time / r32(0.25);
        }
        self.active_highlight.1 = self.active_highlight.1.clamp(Time::ZERO, Time::ONE);

        let highlight_range = model
            .grid
            .get_tile(self.active_highlight.0)
            .and_then(|tile| {
                let range = match tile.tile {
                    Tile::Light(_) => Some(assets.config.light_radius),
                    Tile::Drainer => Some(assets.config.drainer_radius),
                    Tile::Cutter(_) => Some(assets.config.cutter_radius),
                    Tile::Sprinkler(_) => Some(1),
                    _ => None,
                };
                range.map(|range| (self.active_highlight.0, range))
            });

        // Grid
        for x in model.grid.bounds.min.x..=model.grid.bounds.max.x {
            for y in model.grid.bounds.min.y..=model.grid.bounds.max.y {
                let pos = vec2(x, y);
                let highlight =
                    highlight_range.is_some_and(|(p, r)| logic::manhattan_distance(pos, p) <= r);
                let texture = &sprites.tile;
                let color = if highlight {
                    let t = self.active_highlight.1.as_f32() * 0.2;
                    Color::new(1.0 + t, 1.0 + t, 1.0 + t, 1.0)
                } else {
                    Color::WHITE
                };
                self.util.draw_on_tile_with(
                    &model.grid_visual,
                    pos,
                    color,
                    texture,
                    &model.camera,
                    framebuffer,
                );
            }
        }

        // Tiles
        let mut positions: Vec<_> = model.grid.all_positions().collect();
        positions.sort_by_key(|pos| -pos.y);
        for pos in positions {
            let Some(tile) = model.grid.get_tile(pos) else {
                continue;
            };
            let texture = sprites.tiles.get_texture(tile.tile);
            let mult = match *tile.tile {
                Tile::Light(connected)
                | Tile::Wire(connected)
                | Tile::Cutter(Cutter {
                    powered: connected, ..
                })
                | Tile::Pipe(connected)
                | Tile::Sprinkler(connected) => {
                    if connected {
                        1.0
                    } else {
                        0.5
                    }
                }
                _ => 1.0,
            };
            let color = Color::new(mult, mult, mult, 1.0);
            self.util.draw_on_tile_with(
                &model.grid_visual,
                pos,
                color,
                texture,
                &model.camera,
                framebuffer,
            );
        }

        let tile_highlight = |pos: vec2<ICoord>,
                              color: Color,
                              framebuffer: &mut ugli::Framebuffer| {
            self.util.draw_on_tile_with(
                &model.grid_visual,
                pos,
                color,
                &sprites.tile_select,
                &model.camera,
                framebuffer,
            );
            if let Some(tile) = model.grid.get_tile(pos) {
                let name = tile.tile.name().to_uppercase();
                let tile_bounds = model.grid_visual.tile_bounds(pos).as_f32();
                let select_size = sprites.tile_select.size().as_f32() / TILE_SIZE_PIXELS.as_f32();
                let select_bounds = tile_bounds.align_aabb(select_size, vec2(0.5, 0.5));
                let position = select_bounds.align_pos(vec2(0.0, 1.0)) + vec2(0.1, 0.0);
                self.util.draw_text(
                    name,
                    position,
                    &assets.fonts.aseprite,
                    TextRenderOptions::new(0.3)
                        .align(vec2(0.0, 0.0))
                        .color(palette.text),
                    &model.camera,
                    framebuffer,
                );
            }
        };
        let ghost_tile =
            |pos: vec2<ICoord>, tile: &Tile, framebuffer: &mut ugli::Framebuffer<'_>| {
                if model.grid.get_tile(pos).is_none() {
                    let texture = sprites.tiles.get_texture(tile);
                    self.util.draw_on_tile_with(
                        &model.grid_visual,
                        pos,
                        Color::new(0.7, 0.7, 0.7, 0.5),
                        texture,
                        &model.camera,
                        framebuffer,
                    );
                }
            };

        let tile_description = |pos: vec2<ICoord>, framebuffer: &mut ugli::Framebuffer<'_>| {
            let Some(tile) = model.grid.get_tile(pos) else {
                return;
            };
            let text = tile.tile.description();
            if text.is_empty() {
                return;
            }

            let pos = model.grid_visual.tile_bounds(pos).as_f32();
            let pos = Aabb2::point(pos.align_pos(vec2(0.0, 0.0)))
                .extend_right(6.0)
                .extend_down(2.2);
            self.util.draw_nine_slice(
                pos,
                Color::new(1.0, 1.0, 1.0, 0.8),
                &sprites.ui_window,
                model.grid_visual.tile_size.y.as_f32() / TILE_SIZE_PIXELS.y as f32,
                &model.camera,
                framebuffer,
            );

            let size = 0.5;
            let pos = pos.extend_uniform(-0.1);
            let lines = crate::util::wrap_text(
                &self.context.assets.fonts.aseprite,
                text,
                pos.width() / size,
            );
            let row = pos.align_aabb(vec2(pos.width(), size), vec2(0.5, 1.0));
            let rows = row.stack(vec2(0.0, -row.height()), lines.len());

            for (line, position) in lines.into_iter().zip(rows) {
                self.util.draw_text(
                    line,
                    position.align_pos(vec2(0.0, 0.5)),
                    &self.context.assets.fonts.aseprite,
                    TextRenderOptions::new(size)
                        .color(crate::util::with_alpha(palette.text, 1.0))
                        .align(vec2(0.0, 0.5)),
                    &model.camera,
                    framebuffer,
                );
            }
        };

        // Drone action
        match model.drone.target {
            DroneTarget::MoveTo(_) => {}
            DroneTarget::Interact(target, _) => {
                tile_highlight(target, Color::WHITE, framebuffer);
            }
            DroneTarget::PlaceTile(target, ref tile) | DroneTarget::BuyTile(target, ref tile) => {
                ghost_tile(target, tile, framebuffer);
                tile_highlight(target, Color::WHITE, framebuffer);
            }
            DroneTarget::KillBug(bug_id) => {
                let bug = model.grid.tiles.iter().find(|(_, tile)| {
                    if let Tile::Bug(bug) = tile
                        && bug.id == bug_id
                    {
                        true
                    } else {
                        false
                    }
                });
                if let Some((&target, _)) = bug {
                    tile_highlight(target, Color::RED, framebuffer);
                }
            }
        }

        // Input state
        if let Some(target) = cursor.grid_pos {
            match input_state {
                InputState::Idle => {
                    let color = if let Some(tile) = model.grid.get_tile(target)
                        && let Tile::Bug(_) = tile.tile
                    {
                        Color::new(0.7, 0.1, 0.1, 0.5)
                    } else {
                        Color::new(0.7, 0.7, 0.7, 0.5)
                    };
                    tile_highlight(target, color, framebuffer);
                    tile_description(target, framebuffer);
                }
                InputState::PlaceTile(tile) | InputState::BuyTile(tile) => {
                    ghost_tile(target, tile, framebuffer);
                }
            }
        }

        // Drone
        let angle = Angle::from_radians(
            model.drone.velocity.x.signum()
                * model.drone.velocity.y.signum()
                * model.drone.velocity.len()
                / assets.config.drone_max_speed
                * r32(0.5),
        );
        self.util.draw_texture_autoscaled(
            model.drone.position,
            angle.as_f32(),
            &sprites.drone,
            &model.camera,
            framebuffer,
        );
    }

    pub fn draw_ui(&mut self, ui: &GameUI, model: &Model, framebuffer: &mut ugli::Framebuffer) {
        let sprites = &self.context.assets.sprites;
        let palette = &self.context.assets.palette;

        let pixel_scale = get_pixel_scale(framebuffer.size());
        let font_size = 12.0 * pixel_scale;

        // Inventory
        self.util.draw_nine_slice(
            ui.inventory.position,
            Color::WHITE,
            &sprites.ui_window,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Inventory items
        for (widget, (tile, count)) in ui.inventory_items.iter().zip(&model.inventory) {
            let texture = &sprites.tiles.get_texture(tile);
            self.ui
                .draw_texture(widget.position, texture, Color::WHITE, 1.0, framebuffer);

            // Count
            let pos = widget.position.align_pos(vec2(0.5, 1.0)) + vec2(0.0, 3.0) * pixel_scale;
            self.util.draw_text(
                count.to_string(),
                pos,
                &self.context.assets.fonts.aseprite,
                TextRenderOptions::new(font_size)
                    .color(palette.text)
                    .align(vec2(0.5, 0.0)),
                &geng::PixelPerfectCamera,
                framebuffer,
            );
        }

        // Shop
        self.util.draw_nine_slice(
            ui.shop.position,
            Color::WHITE,
            &sprites.ui_window,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Shop items
        for (widget, tile) in &ui.shop_items {
            let unlock_cost = model
                .config
                .shop
                .iter()
                .find(|item| {
                    item.tile == *tile
                        && item.unlocked_at > 0
                        && !model.unlocked_shop.contains(tile)
                })
                .map(|item| item.unlocked_at);

            let texture = &sprites.tiles.get_texture(tile);
            let color = if unlock_cost.is_some() {
                Color::new(0.5, 0.5, 0.5, 1.0)
            } else {
                Color::WHITE
            };
            self.ui
                .draw_texture(widget.position, texture, color, 1.0, framebuffer);

            // Cost
            let (cost, pos) = match unlock_cost {
                Some(unlock) => (unlock, widget.position.center()),
                None => (
                    model.config.get_cost(tile),
                    widget.position.align_pos(vec2(0.5, 1.0)) + vec2(0.0, 3.0) * pixel_scale,
                ),
            };
            self.util.draw_text_gold(
                cost,
                pos,
                &self.context.assets.fonts.aseprite,
                TextRenderOptions::new(font_size)
                    .color(palette.text)
                    .align(vec2(0.5, 0.0)),
                &geng::PixelPerfectCamera,
                framebuffer,
            );
        }

        // Gold
        self.util.draw_nine_slice(
            ui.gold.position,
            Color::WHITE,
            &sprites.ui_window,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );
        let pos = ui.gold.position.extend_uniform(-0.0 * pixel_scale);
        self.util.draw_text_gold(
            model.money,
            pos.center(),
            &self.context.assets.fonts.aseprite,
            TextRenderOptions::new(font_size)
                .color(palette.text)
                .align(vec2(0.5, 0.5)),
            &geng::PixelPerfectCamera,
            framebuffer,
        );
    }
}
