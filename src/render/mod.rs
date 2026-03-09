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
}

impl GameRender {
    pub fn new(context: Context) -> Self {
        Self {
            util: UtilRender::new(context.clone()),
            ui: UiRender::new(context.clone()),
            context,
        }
    }

    pub fn draw_game(
        &mut self,
        model: &Model,
        cursor: &CursorState,
        input_state: &InputState,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let assets = &self.context.assets;
        let sprites = &assets.sprites;
        let palette = &assets.palette;

        // Grid
        for x in -20..20 {
            for y in -10..30 {
                self.util.draw_on_tile(
                    &model.grid_visual,
                    vec2(x, y),
                    &sprites.tile,
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
                Tile::Light(power) | Tile::Wire(power) => {
                    if power {
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
                        .color(crate::util::with_alpha(palette.text, color.a)),
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
        match input_state {
            InputState::Idle => {
                let target = cursor.grid_pos;
                let color = if let Some(tile) = model.grid.get_tile(target)
                    && let Tile::Bug(_) = tile.tile
                {
                    Color::new(0.7, 0.1, 0.1, 0.5)
                } else {
                    Color::new(0.7, 0.7, 0.7, 0.5)
                };
                tile_highlight(target, color, framebuffer);
            }
            InputState::PlaceTile(tile) | InputState::BuyTile(tile) => {
                ghost_tile(cursor.grid_pos, tile, framebuffer);
            }
        }

        // Drone
        let angle = Angle::from_radians(
            model.drone.velocity.x.signum()
                * model.drone.velocity.y.signum()
                * model.drone.velocity.len()
                / r32(Drone::MAX_SPEED)
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
