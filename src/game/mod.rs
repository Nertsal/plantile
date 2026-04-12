mod ui;

pub use self::ui::*;

use crate::{model::*, prelude::*, render::*, ui::context::UiContext, util::SecondOrderState};

const ZOOM_MIN: f32 = -10.0;
const ZOOM_MAX: f32 = 25.0;

const CLICK_MAX_DISTANCE: f64 = 10.0;
const CLICK_MAX_DURATION: f32 = 0.5;

const FIXED_FPS: f32 = 30.0;
const FIXED_DELTA_TIME: f32 = FIXED_FPS.recip();
const MAX_DELTA_TIME: f32 = 0.25;

pub struct GameState {
    context: Context,
    ui_context: UiContext,
    framebuffer_size: vec2<usize>,
    delta_time: Time,
    real_time: Time,
    model_update_timer: f32,
    focus_ui: bool,

    render: GameRender,
    model: Model,
    ui: GameUI,

    hide_ui: bool,
    cursor: CursorState,
    input_state: InputState,
    drag: Option<Drag>,
    zoom: SecondOrderState<f32>,
}

pub struct Drag {
    pub from_world: vec2<FCoord>,
    pub from_screen: vec2<f64>,
    pub from_real_time: Time,
    pub has_moved: bool,
    pub target: DragTarget,
    /// Changes input state if the drag has no effect.
    pub next_state: Option<InputState>,
}

pub enum DragTarget {
    /// Pan camera around.
    Camera,
    /// Cancel hovered queued actions.
    CancelQueued,
    Interact,
    PlaceTile(TileKind),
    BuyTile(TileKind),
}

pub struct CursorState {
    pub screen_pos: vec2<f64>,
    pub world_pos: vec2<FCoord>,
    pub grid_pos: Option<vec2<ICoord>>,
    /// If LMB down, screen position and real time of the initial press.
    pub pressed: Option<(vec2<f64>, Time)>,
}

#[derive(Debug)]
pub enum InputState {
    Idle,
    PlaceTile(TileKind),
    BuyTile(TileKind),
}

impl GameState {
    pub fn new(context: Context) -> Self {
        context.music.play(&context.assets.music.dewdrop, true);

        let mut game = Self {
            render: GameRender::new(context.clone()),
            model: Model::new(context.clone(), context.assets.config.clone()),
            ui: GameUI::new(&context),

            hide_ui: false,
            cursor: CursorState {
                screen_pos: vec2::ZERO,
                world_pos: vec2::ZERO,
                grid_pos: None,
                pressed: None,
            },
            input_state: InputState::Idle,
            drag: None,
            zoom: SecondOrderState::new(3.0, 1.0, 0.0, ZOOM_MAX),

            ui_context: UiContext::new(context.clone()),
            framebuffer_size: vec2(1, 1),
            delta_time: Time::new(0.1),
            real_time: Time::ZERO,
            model_update_timer: 0.0,
            focus_ui: false,
            context,
        };
        game.zoom.target = 0.001; // For better pixels (slight misaligned)
        game
    }

    fn click(&mut self, drag: Option<Drag>) {
        if self.focus_ui {
            // Focus UI first
            return;
        }

        if let Some(target) = self.cursor.grid_pos
            && let InputState::Idle = self.input_state
        {
            let interaction = self.model.interact_with(target, true);
            if dbg!(interaction).is_none()
                && let Some(drag) = drag
                && let Some(state) = dbg!(drag.next_state)
            {
                self.input_state = state;
            }
        }
    }

    fn middle_click(&mut self, _drag: Option<Drag>) {
        if self.focus_ui {
            return;
        }

        if let Some(target) = self.cursor.grid_pos
            && let Some(tile) = self.model.grid.get_tile(target)
        {
            let kind = tile.tile.kind.clone().normalized();
            if self.model.can_place_tile(&kind, true) {
                self.input_state = InputState::PlaceTile(kind);
            }
        }
    }

    fn start_drag(&mut self, target: DragTarget, next_state: Option<InputState>) {
        self.drag = Some(Drag {
            from_world: match target {
                DragTarget::Camera => self.model.camera.center.as_r32(),
                _ => self.cursor.world_pos,
            },
            from_screen: self.cursor.screen_pos,
            from_real_time: self.real_time,
            has_moved: false,
            target,
            next_state,
        });
        self.update_drag();
    }

    fn update_drag(&mut self) {
        let Some(drag) = &mut self.drag else {
            return;
        };
        if drag.from_screen != self.cursor.screen_pos {
            drag.has_moved = true;
        }
        match &drag.target {
            DragTarget::Camera => {
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
            DragTarget::CancelQueued => {
                if drag.has_moved {
                    self.cancel_queued();
                }
            }
            DragTarget::Interact => {
                if let Some(target) = self.cursor.grid_pos {
                    self.model.interact_with(target, false);
                }
            }
            DragTarget::PlaceTile(tile) => {
                if let Some(target) = self.cursor.grid_pos {
                    self.model.place_tile(target, tile.clone());
                    if !self.model.can_place_tile(tile, true) {
                        self.input_state = InputState::Idle;
                        self.drag = None;
                    }
                }
            }
            DragTarget::BuyTile(tile) => {
                if let Some(target) = self.cursor.grid_pos {
                    self.model.buy_tile(target, tile.clone());
                    if !self.model.can_buy_tile(tile, true) {
                        self.input_state = InputState::Idle;
                        self.drag = None;
                    }
                }
            }
        }
    }

    /// Called the first frame that the LMB is pressed down.
    fn lmb_press(&mut self) {
        if self.focus_ui {
            // Focus UI first
            return;
        }

        if let Some(target) = self.cursor.grid_pos {
            if self
                .model
                .grid
                .get_tile(target)
                .is_some_and(|tile| !matches!(tile.tile.state, TileState::Despawning { .. }))
            {
                // Clicked on an active tile
                if self.model.interact_with(target, false).is_some() {
                    self.start_drag(DragTarget::Interact, None);
                }
            } else {
                match &self.input_state {
                    InputState::Idle => {
                        // Clicked on empty space or inactive tile
                        self.model.interact_with(target, false);
                        self.start_drag(DragTarget::Interact, None);
                    }
                    InputState::PlaceTile(tile) => {
                        self.model.place_tile(target, tile.clone());
                        if self.model.can_place_tile(tile, true) {
                            self.start_drag(DragTarget::PlaceTile(tile.clone()), None);
                        } else {
                            self.input_state = InputState::Idle;
                        }
                    }
                    InputState::BuyTile(tile) => {
                        self.model.buy_tile(target, tile.clone());
                        if self.model.can_buy_tile(tile, true) {
                            self.start_drag(DragTarget::BuyTile(tile.clone()), None);
                        } else {
                            self.input_state = InputState::Idle;
                        }
                    }
                }
            }
        }
    }

    fn cancel(&mut self) {
        if self.cancel_queued() {
            return;
        }
        if !matches!(self.input_state, InputState::Idle) {
            // Stop tile placement
            self.input_state = InputState::Idle;
        }
    }

    fn cancel_queued(&mut self) -> bool {
        let Some(target) = self.cursor.grid_pos else {
            return false;
        };

        // Cancel hovered action
        if let Some((i, _)) = self.model.active_action_at(target) {
            match i {
                ActionId::Drone => self.model.drone.target = None,
                ActionId::Queued(i) => {
                    self.model.queued_actions.remove(i);
                }
            }
            true
        } else {
            false
        }
    }

    fn handle_game_events(&mut self, events: impl IntoIterator<Item = GameEvent>) {
        let mut sfx = LinearMap::new();
        for event in events {
            match event {
                GameEvent::Sfx(pos, sound) => {
                    let distance = crate::model::logic::manhattan_distance(
                        self.model
                            .grid_visual
                            .world_to_grid(self.model.camera.center.as_r32()),
                        pos,
                    );
                    let fov = self.model.camera.fov.value();
                    let max_distance = (fov * 3.0).min(50.0);
                    let volume = (1.0
                        - (distance as f32 * self.model.grid_visual.tile_size.y.as_f32()
                            - fov / 4.0)
                            / max_distance)
                        .clamp(0.0, 1.0);
                    let volume = volume.sqrt();
                    let v = sfx.entry(sound).or_insert(0.0);
                    *v = volume.max(*v);
                }
            }
        }

        let sounds = &self.context.assets.sounds;
        for (sfx, volume) in sfx {
            let sfx = match sfx {
                GameSfx::TileBuild => &sounds.tile_build,
                GameSfx::RockSpawn => &sounds.rock_spawn,
                GameSfx::SeedTakeEnergy => &sounds.water_consume,
                GameSfx::PlantGrowth => &sounds.plant_growth,
                GameSfx::PlantHarvest => &sounds.bug_eat,

                GameSfx::WaterSpawn => &sounds.water_spawn,
                GameSfx::WaterConsume => &sounds.water_consume,
                GameSfx::WaterSprinkle => &sounds.water_sprinkle,
                GameSfx::WaterEvaporate => &sounds.evaporate,

                GameSfx::BugSpawn => &sounds.bug_spawn,
                GameSfx::BugMove => &sounds.bug_move,
                GameSfx::BugEat => &sounds.bug_eat,
                GameSfx::BugPoop => &sounds.bug_poop,
                GameSfx::PoopConsume => &sounds.water_consume,
                GameSfx::PoopDespawn => &sounds.evaporate,
            };
            self.context.sfx.play_volume(sfx, volume);
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

        self.update_drag();

        let mut delta_time = Time::new(delta_time as f32);
        self.delta_time = delta_time;
        self.real_time += delta_time;

        let geng = self.context.geng.clone();
        let window = geng.window();

        if cfg!(feature = "cheats")
            && window.is_key_pressed(geng::Key::T)
            && window.is_key_pressed(geng::Key::ShiftLeft)
        {
            delta_time *= r32(20.0);
        }

        // Update game
        self.model_update_timer -= delta_time.as_f32();
        while self.model_update_timer < 0.0 {
            self.model.fixed_update(r32(FIXED_DELTA_TIME));
            self.model_update_timer += FIXED_DELTA_TIME;
        }

        let mut model_delta_time = delta_time;
        while model_delta_time.as_f32() > MAX_DELTA_TIME {
            self.model.update(r32(MAX_DELTA_TIME));
            model_delta_time -= r32(MAX_DELTA_TIME);
        }
        self.model.update(delta_time);

        // Game events
        let events = std::mem::take(&mut self.model.events);
        self.handle_game_events(events);

        // UI events
        for (widget, (tile, _)) in self
            .ui
            .inventory_items
            .iter_mut()
            .zip(&self.model.inventory)
        {
            if widget.mouse_left.just_pressed && self.model.can_place_tile(tile, true) {
                self.input_state = InputState::PlaceTile(tile.clone());
                widget.hovered_time = Some(0.0);
                break;
            }
        }
        for (widget, tile) in &mut self.ui.shop_items {
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
            if widget.mouse_left.just_pressed {
                if let Some(unlock) = unlock_cost {
                    if self.model.money >= unlock {
                        self.model.money -= unlock;
                        self.model.unlocked_shop.push(tile.clone());
                        widget.hovered_time = Some(0.0);
                    }
                } else if self.model.can_buy_tile(tile, true) {
                    self.input_state = InputState::BuyTile(tile.clone());
                    widget.hovered_time = Some(0.0);
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
            geng::Event::KeyPress { key: geng::Key::F1 } => {
                self.hide_ui = !self.hide_ui;
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
            geng::Event::MousePress { button } => match button {
                geng::MouseButton::Middle | geng::MouseButton::Right => {
                    if let geng::MouseButton::Right = button
                        && let Some(grid_pos) = self.cursor.grid_pos
                        && self.model.active_action_at(grid_pos).is_some()
                    {
                        self.drag = Some(Drag {
                            from_world: self.cursor.world_pos,
                            from_screen: self.cursor.screen_pos,
                            from_real_time: self.real_time,
                            has_moved: false,
                            target: DragTarget::CancelQueued,
                            next_state: None,
                        });
                    } else {
                        self.drag = Some(Drag {
                            from_world: self.model.camera.center.as_r32(),
                            from_screen: self.cursor.screen_pos,
                            from_real_time: self.real_time,
                            has_moved: false,
                            target: DragTarget::Camera,
                            next_state: None,
                        });
                    }
                }
                geng::MouseButton::Left => {
                    self.cursor.pressed = Some((self.cursor.screen_pos, self.real_time));
                    self.lmb_press();
                }
            },
            geng::Event::MouseRelease { button } => {
                match button {
                    geng::MouseButton::Middle => {
                        // Stop dragging camera
                        if let Some(drag) = &self.drag
                            && let DragTarget::Camera = drag.target
                            && let Some(drag) = self.drag.take()
                            && (self.cursor.screen_pos - drag.from_screen).len_sqr()
                                < CLICK_MAX_DISTANCE
                            && (self.real_time - drag.from_real_time).as_f32() < CLICK_MAX_DURATION
                        {
                            // Short middle click
                            self.middle_click(Some(drag));
                        }
                    }
                    geng::MouseButton::Right => {
                        // Stop dragging
                        if let Some(drag) = self.drag.take()
                            && (self.cursor.screen_pos - drag.from_screen).len_sqr()
                                < CLICK_MAX_DISTANCE
                            && (self.real_time - drag.from_real_time).as_f32() < CLICK_MAX_DURATION
                        {
                            // Short right click - cancel action
                            self.cancel();
                        }
                    }
                    geng::MouseButton::Left => {
                        let drag = self.drag.take();
                        if let Some((from_screen, from_real_time)) = self.cursor.pressed
                            && (self.cursor.screen_pos - from_screen).len_sqr() < CLICK_MAX_DISTANCE
                            && (self.real_time - from_real_time).as_f32() < CLICK_MAX_DURATION
                        {
                            // Short left click
                            self.click(drag);
                        }
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
        self.focus_ui = self.ui.inventory.hovered || self.ui.shop.hovered;

        self.render.draw_game(
            &self.model,
            &self.cursor,
            &self.input_state,
            self.hide_ui,
            self.focus_ui,
            framebuffer,
            self.delta_time,
        );
        if !self.hide_ui {
            self.render.draw_ui(
                &self.ui,
                &self.model,
                &self.input_state,
                &self.ui_context,
                framebuffer,
            );
        }

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
