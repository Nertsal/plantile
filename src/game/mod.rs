mod ui;

pub use self::ui::*;

use crate::{model::*, prelude::*, render::*, ui::context::UiContext, util::SecondOrderState};

const ZOOM_MIN: f32 = -5.0;
const ZOOM_MAX: f32 = 15.0;

pub struct GameState {
    context: Context,
    ui_context: UiContext,
    framebuffer_size: vec2<usize>,
    delta_time: Time,
    real_time: Time,

    render: GameRender,
    model: Model,
    ui: GameUI,

    cursor: CursorState,
    input_state: InputState,
    camera_drag: Option<Drag>,
    zoom: SecondOrderState<f32>,
}

pub struct Drag {
    pub from_world: vec2<FCoord>,
    pub from_screen: vec2<f64>,
    pub from_real_time: Time,
}

pub struct CursorState {
    pub screen_pos: vec2<f64>,
    pub world_pos: vec2<FCoord>,
    pub grid_pos: Option<vec2<ICoord>>,
}

#[derive(Debug)]
pub enum InputState {
    Idle,
    PlaceTile(TileKind),
    BuyTile(TileKind),
}

impl GameState {
    pub fn new(context: Context) -> Self {
        let mut game = Self {
            render: GameRender::new(context.clone()),
            model: Model::new(context.clone(), context.assets.config.clone()),
            ui: GameUI::new(&context),

            cursor: CursorState {
                screen_pos: vec2::ZERO,
                world_pos: vec2::ZERO,
                grid_pos: None,
            },
            input_state: InputState::Idle,
            camera_drag: None,
            zoom: SecondOrderState::new(3.0, 1.0, 0.0, ZOOM_MAX),

            ui_context: UiContext::new(context.clone()),
            framebuffer_size: vec2(1, 1),
            delta_time: Time::new(0.1),
            real_time: Time::ZERO,
            context,
        };
        game.zoom.target = 0.001; // For better pixels (slight misaligned)
        game
    }

    fn left_click(&mut self) {
        if self.ui.inventory.hovered || self.ui.shop.hovered {
            // Focus UI first
            return;
        }

        if let Some(target) = self.cursor.grid_pos {
            match &self.input_state {
                InputState::Idle => {
                    self.model.interact_with(target);
                }
                InputState::PlaceTile(tile) => {
                    if self.model.place_tile(target, tile.clone()) {
                        self.input_state = InputState::Idle;
                    }
                }
                InputState::BuyTile(tile) => {
                    if self.model.buy_tile(target, tile.clone()) {
                        self.input_state = InputState::Idle;
                    }
                }
            }
        }
    }
}

impl geng::State for GameState {
    fn update(&mut self, delta_time: f64) {
        self.ui_context.update(delta_time as f32);

        {
            // Camera Zoom
            let scroll = self.ui_context.cursor.scroll;
            if scroll.abs() > 0.01 {
                let sensitivity = 90.0;
                self.zoom.target -= scroll.signum() * sensitivity * delta_time as f32;
                self.zoom.target = self.zoom.target.clamp(ZOOM_MIN, ZOOM_MAX);
            }
            self.zoom.update(delta_time as f32);
            self.model.camera.fov = Camera2dFov::MinSide(15.0 + self.zoom.current);
        }
        if let Some(drag) = &self.camera_drag {
            let from = self
                .model
                .camera
                .screen_to_world(self.framebuffer_size.as_f32(), drag.from_screen.as_f32());
            let to = self.model.camera.screen_to_world(
                self.framebuffer_size.as_f32(),
                self.cursor.screen_pos.as_f32(),
            );
            self.model.camera.center = drag.from_world.as_f32() + from - to;
        }

        let mut delta_time = Time::new(delta_time as f32);
        self.delta_time = delta_time;
        self.real_time += delta_time;

        if cfg!(feature = "cheats")
            && self.context.geng.window().is_key_pressed(geng::Key::T)
            && self
                .context
                .geng
                .window()
                .is_key_pressed(geng::Key::ShiftLeft)
        {
            delta_time *= r32(20.0);
        }
        self.model.update(delta_time);

        // UI events
        for (widget, (tile, _)) in self.ui.inventory_items.iter().zip(&self.model.inventory) {
            if widget.mouse_left.clicked {
                self.input_state = InputState::PlaceTile(tile.clone());
                break;
            }
        }
        for (widget, tile) in &self.ui.shop_items {
            let unlock_cost = self
                .model
                .config
                .shop
                .iter()
                .find(|item| {
                    item.tile == *tile
                        && item.unlocked_at > 0
                        && !self.model.unlocked_shop.contains(tile)
                })
                .map(|item| item.unlocked_at);
            let cost = self.model.config.get_cost(tile);
            if widget.mouse_left.clicked {
                if let Some(unlock) = unlock_cost {
                    if self.model.money >= unlock {
                        self.model.money -= unlock;
                        self.model.unlocked_shop.push(tile.clone());
                    }
                } else if self.model.money >= cost {
                    self.input_state = InputState::BuyTile(tile.clone());
                }
                break;
            }
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            #[cfg(feature = "cheats")]
            geng::Event::KeyPress { key: geng::Key::G }
                if self
                    .context
                    .geng
                    .window()
                    .is_key_pressed(geng::Key::ShiftLeft) =>
            {
                self.model.money += 10000;
            }
            geng::Event::Wheel { delta } => {
                self.ui_context.cursor.scroll += delta as f32;
            }
            geng::Event::CursorMove { position } => {
                self.ui_context.cursor.cursor_move(position.as_f32());
                self.cursor.screen_pos = position;
                self.cursor.world_pos = self
                    .model
                    .camera
                    .screen_to_world(self.framebuffer_size.as_f32(), position.as_f32())
                    .as_r32();
                let grid_pos = self.model.grid_visual.world_to_grid(self.cursor.world_pos);
                self.cursor.grid_pos = self.model.grid.in_bounds(grid_pos).then_some(grid_pos);
            }
            geng::Event::MousePress {
                button: geng::MouseButton::Left,
            } => {
                self.left_click();
            }
            geng::Event::MousePress {
                button: geng::MouseButton::Middle | geng::MouseButton::Right,
            } => {
                self.camera_drag = Some(Drag {
                    from_world: self.model.camera.center.as_r32(),
                    from_screen: self.cursor.screen_pos,
                    from_real_time: self.real_time,
                });
            }
            geng::Event::MouseRelease { button } => {
                if let geng::MouseButton::Right | geng::MouseButton::Middle = button
                    && let Some(drag) = self.camera_drag.take() // Stop dragging camera
                    && let geng::MouseButton::Right = button
                    && (self.cursor.screen_pos - drag.from_screen).len_sqr() < 5.0
                    && (self.real_time - drag.from_real_time).as_f32() < 0.5
                {
                    // Short right click - cancel action
                    if !matches!(self.input_state, InputState::Idle) {
                        // Stop tile placement
                        self.input_state = InputState::Idle;
                    } else {
                        // Stop drone action
                        self.model.drone.target = DroneTarget::MoveTo(
                            self.model
                                .grid_visual
                                .world_to_grid(self.model.drone.position),
                        );
                    }
                }
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
            &self.model,
            Aabb2::ZERO.extend_positive(framebuffer.size().as_f32()),
            &mut self.ui_context,
        );
        self.ui_context.frame_end();

        self.render.draw_game(
            &self.model,
            &self.cursor,
            &self.input_state,
            framebuffer,
            self.delta_time,
        );
        self.render.draw_ui(&self.ui, &self.model, framebuffer);

        // Debug
        // self.render.util.draw_text(
        //     crate::model::logic::density_near(self.cursor_grid_pos, &self.model.grid).to_string(),
        //     vec2(0.0, 0.0),
        //     &self.context.assets.fonts.default,
        //     util::TextRenderOptions::new(1.0),
        //     &self.model.camera,
        //     framebuffer,
        // );
    }
}
