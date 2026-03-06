pub mod util;

use crate::{model::Model, prelude::*};

type Color = Rgba<f32>;

pub struct GameRender {
    context: Context,
}

impl GameRender {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    pub fn draw_game(&mut self, model: &Model, framebuffer: &mut ugli::Framebuffer) {
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
}
