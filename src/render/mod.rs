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
            self.util
                .draw_on_tile(&model.grid_visual, pos, texture, &model.camera, framebuffer);
        }

        let tile_highlight =
            |pos: vec2<ICoord>, color: Color, framebuffer: &mut ugli::Framebuffer| {
                self.util.draw_on_tile_with(
                    &model.grid_visual,
                    pos,
                    color,
                    &sprites.tile_select,
                    &model.camera,
                    framebuffer,
                );
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
        }

        // Input state
        match input_state {
            InputState::Idle => {
                tile_highlight(cursor.grid_pos, Color::new(0.7, 0.7, 0.7, 0.5), framebuffer);
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
                &self.context.assets.fonts.default,
                TextRenderOptions::new(20.0 * pixel_scale)
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
            &sprites.ui_window_shop,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Shop items
        for (widget, tile) in &ui.shop_items {
            let texture = &sprites.tiles.get_texture(tile);
            self.ui
                .draw_texture(widget.position, texture, Color::WHITE, 1.0, framebuffer);

            // Price
            let price = 20;
            let pos = widget.position.align_pos(vec2(0.5, 1.0)) + vec2(0.0, 3.0) * pixel_scale;
            self.util.draw_text(
                format!("{}g", price),
                pos,
                &self.context.assets.fonts.default,
                TextRenderOptions::new(20.0 * pixel_scale)
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
        self.util.draw_text(
            format!("{}g", model.money),
            pos.center(),
            &self.context.assets.fonts.default,
            TextRenderOptions::new(20.0 * pixel_scale)
                .color(palette.text)
                .align(vec2(0.5, 0.5)),
            &geng::PixelPerfectCamera,
            framebuffer,
        );
    }
}
