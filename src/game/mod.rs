use crate::{model::*, prelude::*};

pub struct GameState {
    context: Context,

    model: Model,
}

impl GameState {
    pub fn new(context: Context) -> Self {
        Self {
            model: Model::new(),

            context,
        }
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None, None);
    }
}
