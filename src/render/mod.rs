pub mod ui;
pub mod util;

use self::{ui::*, util::*};

use crate::{game::GameUI, model::Model, prelude::*, ui::layout::AreaOps};

pub struct GameRender {
    context: Context,
    util: UtilRender,
    ui: UiRender,
}

impl GameRender {
    pub fn new(context: Context) -> Self {
        Self {
            util: UtilRender::new(context.clone()),
            ui: UiRender::new(context.clone()),
            context,
        }
    }

    pub fn draw_game(&mut self, model: &Model, framebuffer: &mut ugli::Framebuffer) {
        let assets = &self.context.assets;
        let palette = &assets.palette;

        // Grid
        for x in -20..20 {
            for y in -10..30 {
                let pos = model.grid_visual.tile_bounds(vec2(x, y));
                let color = if model.grid.is_tile_lit(vec2(x, y)) {
                    palette.tile_lit
                } else {
                    palette.tile_dark
                };
                self.context
                    .geng
                    .draw2d()
                    .quad(framebuffer, &model.camera, pos.as_f32(), color);
            }
        }

        // Lights
        for light in &model.grid.lights {
            let color = palette.light;
            let pos = model.grid_visual.multitile_bounds(light.pos);
            self.context
                .geng
                .draw2d()
                .quad(framebuffer, &model.camera, pos.as_f32(), color);
        }

        // Plants
        for plant in &model.grid.plants {
            let color = Color::GREEN;
            for pos in itertools::chain![
                [plant.root],
                plant.stem.iter().copied(),
                plant.leaves.iter().copied()
            ] {
                let tile = model.grid_visual.tile_bounds(pos).as_f32();
                self.context
                    .geng
                    .draw2d()
                    .quad(framebuffer, &model.camera, tile, color);
            }
        }
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

        self.ui
            .draw_quad(ui.scissors.position, palette.tile_lit, framebuffer);
        self.ui.draw_texture(
            ui.scissors.position,
            &sprites.scissors,
            Color::WHITE,
            1.0,
            framebuffer,
        );

        self.ui
            .draw_quad(ui.seed.position, palette.tile_lit, framebuffer);
        self.ui.draw_texture(
            ui.seed.position,
            &sprites.seed,
            Color::WHITE,
            1.0,
            framebuffer,
        );
    }
}
