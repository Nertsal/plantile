pub mod ui;
pub mod util;

use self::{ui::*, util::*};

use crate::{
    game::{CursorState, GameUI},
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
            let texture = match tile.tile {
                Tile::Leaf(_) => &sprites.tiles.plant,
                Tile::Light => &sprites.tiles.light,
            };
            self.util
                .draw_on_tile(&model.grid_visual, pos, texture, &model.camera, framebuffer);
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

        // Cursor selection
        self.util.draw_on_tile(
            &model.grid_visual,
            cursor.grid_pos,
            &sprites.tile_select,
            &model.camera,
            framebuffer,
        );
    }

    pub fn draw_ui(&mut self, ui: &GameUI, model: &Model, framebuffer: &mut ugli::Framebuffer) {
        let sprites = &self.context.assets.sprites;
        let palette = &self.context.assets.palette;

        self.ui.draw_texture(
            ui.coins
                .position
                .align_aabb(vec2(1.0, 1.0) * ui.coins.position.height(), vec2(0.0, 0.5)),
            &sprites.coin,
            Color::WHITE,
            1.0,
            framebuffer,
        );
        self.util.draw_text(
            model.money.to_string(),
            ui.coins.position.align_pos(vec2(0.0, 0.5))
                + vec2(ui.coins.position.height() * 0.75, 0.0),
            &self.context.assets.fonts.default,
            TextRenderOptions::new(ui.coins.position.height() * 0.6)
                .color(palette.text)
                .align(vec2(0.0, 0.5)),
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // self.ui
        //     .draw_quad(ui.scissors.position, Color::GRAY, framebuffer);
        // self.ui.draw_texture(
        //     ui.scissors.position,
        //     &sprites.scissors,
        //     Color::WHITE,
        //     1.0,
        //     framebuffer,
        // );

        // self.ui
        //     .draw_quad(ui.seed.position, Color::GRAY, framebuffer);
        // self.ui.draw_texture(
        //     ui.seed.position,
        //     &sprites.seed,
        //     Color::WHITE,
        //     1.0,
        //     framebuffer,
        // );
    }
}
