pub mod ui;
pub mod util;

use self::{ui::*, util::*};

use crate::{game::GameUI, model::Model, prelude::*};

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
        self.ui.draw_texture(
            ui.scissors.position,
            &sprites.scissors,
            Color::WHITE,
            1.0,
            framebuffer,
        );
    }
}
