mod ui;

pub use self::ui::*;

use crate::{model::*, prelude::*, render::*, ui::context::UiContext};

pub struct GameState {
    context: Context,
    ui_context: UiContext,
    framebuffer_size: vec2<usize>,

    render: GameRender,
    model: Model,
    ui: GameUI,

    cursor_pos: vec2<f64>,
    cursor_world_pos: vec2<FCoord>,
    cursor_grid_pos: vec2<ICoord>,
    input_state: InputState,
}

#[derive(Debug)]
pub enum InputState {
    Idle,
    UseItem(Item),
}

#[derive(Debug, Clone)]
pub enum Item {
    Scissors,
}

impl GameState {
    pub fn new(context: Context) -> Self {
        Self {
            render: GameRender::new(context.clone()),
            model: Model::new(),
            ui: GameUI::new(),

            cursor_pos: vec2::ZERO,
            cursor_world_pos: vec2::ZERO,
            cursor_grid_pos: vec2::ZERO,
            input_state: InputState::Idle,

            ui_context: UiContext::new(context.clone()),
            framebuffer_size: vec2(1, 1),
            context,
        }
    }

    fn left_click(&mut self) {
        if let InputState::UseItem(item) = &self.input_state {
            let target = self.cursor_grid_pos;
            match item {
                Item::Scissors => {
                    if let Some(plant) = self.model.grid.plants.iter_mut().find(|plant| {
                        plant.stem.contains(&target) || plant.leaves.contains(&target)
                    }) {
                        plant.stem.clear();
                        plant.leaves.clear();
                    }
                }
            }
        }
    }
}

impl geng::State for GameState {
    fn update(&mut self, delta_time: f64) {
        self.ui_context.update(delta_time as f32);

        let delta_time = Time::new(delta_time as f32);
        self.model.update(delta_time);

        match &mut self.input_state {
            InputState::Idle => {
                if self.ui.scissors.mouse_left.clicked {
                    self.input_state = InputState::UseItem(Item::Scissors);
                }
            }
            InputState::UseItem(_) => {}
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::Wheel { delta } => {
                self.ui_context.cursor.scroll += delta as f32;
            }
            geng::Event::CursorMove { position } => {
                self.ui_context.cursor.cursor_move(position.as_f32());
                self.cursor_pos = position;
                self.cursor_world_pos = self
                    .model
                    .camera
                    .screen_to_world(self.framebuffer_size.as_f32(), position.as_f32())
                    .as_r32();
                self.cursor_grid_pos = self.model.grid_visual.world_to_grid(self.cursor_world_pos);
            }
            geng::Event::MousePress {
                button: geng::MouseButton::Left,
            } => {
                self.left_click();
            }
            _ => {}
        }
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
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
