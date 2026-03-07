mod ui;

pub use self::ui::*;

use crate::{model::*, prelude::*, render::*, ui::context::UiContext};

pub struct GameState {
    context: Context,
    ui_context: UiContext,

    render: GameRender,
    model: Model,
    ui: GameUI,
}

impl GameState {
    pub fn new(context: Context) -> Self {
        Self {
            render: GameRender::new(context.clone()),
            model: Model::new(),
            ui: GameUI::new(),

            ui_context: UiContext::new(context.clone()),
            context,
        }
    }
}

impl geng::State for GameState {
    fn update(&mut self, delta_time: f64) {
        self.ui_context.update(delta_time as f32);

        let delta_time = Time::new(delta_time as f32);
        self.model.update(delta_time);
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::Wheel { delta } => {
                self.ui_context.cursor.scroll += delta as f32;
            }
            geng::Event::CursorMove { position } => {
                self.ui_context.cursor.cursor_move(position.as_f32());
            }
            _ => {}
        }
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(
            framebuffer,
            Some(self.context.assets.palette.background),
            None,
            None,
        );

        self.ui.layout(
            Aabb2::ZERO.extend_positive(framebuffer.size().as_f32()),
            &mut self.ui_context,
        );
        self.ui_context.frame_end();

        self.render.draw_game(&self.model, framebuffer);
        self.render.draw_ui(&self.ui, &self.model, framebuffer);
    }
}
