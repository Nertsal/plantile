use crate::{model::*, prelude::*, render::*};

pub struct GameState {
    context: Context,

    model: Model,
    render: GameRender,
}

impl GameState {
    pub fn new(context: Context) -> Self {
        Self {
            model: Model::new(),
            render: GameRender::new(context.clone()),

            context,
        }
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None, None);

        self.render.draw_game(&self.model, framebuffer);
    }
}
