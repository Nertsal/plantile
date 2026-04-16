pub mod ui;
pub mod util;

use self::{ui::*, util::*};

use crate::{
    game::{CursorState, GameUI, InputState},
    model::*,
    prelude::*,
    ui::{context::UiContext, layout::AreaOps},
};

/// Full size of a single tile in pixels, used for scaling textures to properly fit on the tile.
const TILE_SIZE_PIXELS: vec2<usize> = vec2(32, 32);
/// Transparency of the queued tile action.
const QUEUED_ALPHA: f32 = 0.75;
/// Transparency of the previewed action.
const HOVER_ALPHA: f32 = 0.5;

/// Brightness of locked shop items.
const SHOP_TILE_LOCKED: f32 = 0.3;
/// Brightness of unlocked, but unaffordable shop items.
const SHOP_TILE_TOO_EXPENSIVE: f32 = 0.5;

/// Duration of the tile hover squish animation.
const HOVER_ANIMATION_TIME: f32 = 0.5;
/// Duration of the hover highlight transition.
const HIGHLIGHT_TRANSITION: f32 = 0.25;
/// Extra brightness of the highlighted tiles.
const HOVER_HIGHLIGHT: f32 = 0.15;
/// Speed of the hover highlighting blinking.
const HOVER_BLINK_SPEED: f32 = 4.0;
/// Relative brightness of the highlighted range for the tiles that match the hovered one.
const OTHER_RANGE_HIGHLIGHT: f32 = 0.4;

pub struct GameRender {
    pub context: Context,
    pub util: UtilRender,
    pub ui: UiRender,
    /// (position, animation (0..1), real time passed)
    pub active_highlight: (vec2<ICoord>, Time, Time),
    pub place_highlight: Option<(TileKind, Time)>,
    pub hover_animation: Vec<(vec2<ICoord>, Time)>,
    pub tile_shake: LinearMap<vec2<ICoord>, (vec2<f32>, bool)>,
}

impl GameRender {
    pub fn new(context: Context) -> Self {
        Self {
            util: UtilRender::new(context.clone()),
            ui: UiRender::new(context.clone()),
            active_highlight: (vec2::ZERO, Time::ZERO, Time::ZERO),
            place_highlight: None,
            hover_animation: Vec::new(),
            tile_shake: LinearMap::new(),
            context,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_game(
        &mut self,
        model: &Model,
        cursor: &CursorState,
        input_state: &InputState,
        hide_ui: bool,
        focus_ui: bool,
        framebuffer: &mut ugli::Framebuffer,
        delta_time: Time,
    ) {
        let assets = self.context.assets.clone();
        let sprites = &assets.sprites;
        let palette = &assets.palette;

        let pixel_scale = model.grid_visual.tile_size.y.as_f32() / TILE_SIZE_PIXELS.y as f32;

        if let Some(selected) = cursor.grid_pos
            && model.grid.get_tile(selected).is_some()
        {
            let push = match self
                .hover_animation
                .iter()
                .position(|(p, _)| *p == selected)
            {
                Some(i) if i + 1 < self.hover_animation.len() => {
                    self.hover_animation.remove(i);
                    true
                }
                Some(_) => false,
                None => true,
            };
            if push {
                self.hover_animation.push((selected, Time::ZERO));
                self.context.sfx.play(&assets.sounds.ui_hover);
            }
        }
        for (_, time) in &mut self.hover_animation {
            *time += delta_time / r32(HOVER_ANIMATION_TIME);
            *time = (*time).clamp(Time::ZERO, Time::ONE);
        }
        self.hover_animation
            .retain(|(p, time)| *time < Time::ONE || Some(*p) == cursor.grid_pos);
        // Update tile shake, if the tile hasnt been shaken in last frame, forget about it
        self.tile_shake
            .retain(|_, (_, flag)| std::mem::replace(flag, false));

        // Update hovered timing
        if Some(self.active_highlight.0) != cursor.grid_pos {
            self.active_highlight.1 -= delta_time / r32(HIGHLIGHT_TRANSITION);
            if self.active_highlight.1 <= Time::ZERO
                && let Some(grid_pos) = cursor.grid_pos
            {
                self.active_highlight.0 = grid_pos;
                self.active_highlight.2 = Time::ZERO; // Reset timer
            }
        } else {
            self.active_highlight.1 += delta_time / r32(HIGHLIGHT_TRANSITION);
            self.active_highlight.2 += delta_time;
        }
        self.active_highlight.1 = self.active_highlight.1.clamp(Time::ZERO, Time::ONE);

        // Update placement timing
        if let InputState::PlaceTile(tile) | InputState::BuyTile(tile) = input_state
            && tile.action_range(&model.config).is_some()
        {
            // Timer go up
            if let Some((kind, t)) = &mut self.place_highlight {
                if kind.name() == tile.name() {
                    *t += delta_time / r32(HIGHLIGHT_TRANSITION);
                } else {
                    *t -= delta_time / r32(HIGHLIGHT_TRANSITION);
                    if t.as_f32() <= 0.0 {
                        *kind = tile.clone();
                    }
                }
                *t = (*t).clamp(R32::ZERO, R32::ONE);
            } else {
                self.place_highlight = Some((tile.clone(), Time::ZERO));
            }
        } else if let Some((_, t)) = &mut self.place_highlight {
            // Timer go down
            *t -= delta_time / r32(HIGHLIGHT_TRANSITION);
            if t.as_f32() <= 0.0 {
                self.place_highlight = None;
            }
        }

        // Tile highlight
        let mut highlight_t = self.active_highlight.1;
        let highlighted_tile = if let Some((tile, t)) = &self.place_highlight {
            // Highlight placement
            highlight_t = *t;
            Some((tile, cursor.grid_pos))
        } else {
            // Highlight hovered
            model
                .grid
                .get_tile(self.active_highlight.0)
                .map(|tile| (&tile.tile.kind, Some(self.active_highlight.0)))
        };
        let highlighted_tiles = if let Some((kind, Some(pos))) = highlighted_tile
            && let TileKind::Leaf(_) | TileKind::Seed(_) = kind
        {
            logic::get_whole_plant(&model.grid, pos, &model.config)
        } else {
            vec![]
        };

        // Highlight empty tiles to visualize action range
        let highlight_range = highlighted_tile.and_then(|(tile, pos)| {
            pos.and_then(|pos| {
                let range = tile.action_range(&model.config);
                range.map(|range| (pos, range, 1.0))
            })
        });
        let highlight_range: Vec<_> = itertools::chain![
            highlight_range,
            model.grid.all_tiles().filter_map(|tile| {
                if Some(tile.tile.kind.name()) == highlighted_tile.map(|(tile, _)| tile.name())
                    && let Some(range) = tile.tile.kind.action_range(&model.config)
                {
                    let powered = tile.tile.kind.is_powered().is_none_or(|p| p);
                    powered.then_some((tile.pos, range, OTHER_RANGE_HIGHLIGHT))
                } else {
                    None
                }
            })
        ]
        .collect();

        // Grid
        for x in model.grid.bounds.min.x..=model.grid.bounds.max.x {
            for y in model.grid.bounds.min.y..=model.grid.bounds.max.y {
                let pos = vec2(x, y);
                let highlight = highlight_range
                    .iter()
                    .filter_map(|&(p, r, a)| {
                        let d = logic::manhattan_distance(pos, p);
                        (d <= r).then_some(r32(a))
                    })
                    .max();
                let texture = &sprites.tile;
                let color = if let Some(a) = highlight {
                    let t = highlight_t.as_f32() * 0.2 * a.as_f32();
                    Color::new(1.0 + t, 1.0 + t, 1.0 + t, 1.0)
                } else {
                    Color::WHITE
                };
                self.util.draw_on_tile(
                    &model.grid_visual,
                    pos,
                    color,
                    texture,
                    &model.camera,
                    framebuffer,
                );
            }
        }

        // Tiles
        let mut positions: Vec<_> = model.grid.all_positions().collect();
        positions.sort_by_key(|pos| {
            let layer = if let Some(tile) = model.grid.get_tile(*pos) {
                if let TileState::Spawning {
                    from_position: Some(_),
                    ..
                }
                | TileState::Despawning {
                    to_position: Some(_),
                    ..
                } = tile.tile.state
                {
                    -1
                } else {
                    0
                }
            } else {
                0
            };
            (layer, (-pos.y, pos.x))
        });
        // TODO: instancing
        for pos in positions {
            let Some(tile) = model.grid.get_tile(pos) else {
                continue;
            };
            let Some(texture) = sprites.tiles.get_texture(&tile.tile.kind) else {
                continue;
            };
            let mut mult = match tile.tile.kind {
                TileKind::Pipe(connected) | TileKind::Sprinkler(connected) => {
                    if connected {
                        1.0
                    } else {
                        0.6
                    }
                }
                TileKind::Leaf(_) | TileKind::Seed(_)
                    if !model.grid.is_tile_lit(pos, &model.config) =>
                {
                    0.6
                }
                _ => {
                    if let Some(false) = tile.tile.kind.is_powered() {
                        0.6
                    } else {
                        1.0
                    }
                }
            };

            let mut transform = mat3::identity();
            let mut shake = false;
            match &tile.tile.state {
                TileState::Spawning {
                    timer,
                    from_position,
                } => {
                    let t = timer.ratio().as_f32();
                    let t = 1.0 - crate::util::ease_out_elastic_with(1.0 - t, 2.0, 1.0);
                    let scale = 1.0 + 0.15 * t;
                    let rotation = -10.0 * t;
                    let pos = from_position.map_or(vec2::ZERO, |from| {
                        (from - model.grid_visual.tile_center(tile.pos)).as_f32() * t
                    });
                    transform *= mat3::translate(pos)
                        * mat3::scale_uniform(scale)
                        * mat3::rotate(Angle::from_degrees(rotation));
                }
                TileState::Transforming(timer) => {
                    let t = timer.ratio().as_f32();
                    let t = 1.0 - crate::util::ease_out_elastic_with(1.0 - t, 2.0, 1.0);
                    let scale = 1.0 + 0.15 * t;
                    let rotation = 10.0 * t;
                    transform *=
                        mat3::scale_uniform(scale) * mat3::rotate(Angle::from_degrees(rotation));
                }
                TileState::Despawning { timer, to_position } => {
                    let t = timer.ratio().as_f32();
                    let t = 1.0 - crate::util::ease_out_elastic_with(1.0 - t, 0.5, 1.0);
                    let scale = 0.9 * t;
                    let rotation = 5.0 + 5.0 * t;
                    let pos = to_position.map_or(vec2::ZERO, |to| {
                        (to - model.grid_visual.tile_center(tile.pos)).as_f32() * (1.0 - t)
                    });
                    transform *= mat3::translate(pos)
                        * mat3::scale_uniform(scale)
                        * mat3::rotate(Angle::from_degrees(rotation));
                }
                TileState::Moving { timer, delta } => {
                    let offset = movement_animation(&model.grid_visual, timer, *delta);
                    transform *= mat3::translate(offset);
                }
                TileState::Idle => {
                    if let Some((_, t)) = self.hover_animation.iter().find(|(p, _)| *p == pos) {
                        // Hover animation
                        let t = t.as_f32();
                        let scale = hover_animation(t);
                        transform *= mat3::scale(scale);
                    }
                }
                TileState::DroneAction => {
                    shake = true;
                }
            }

            // Tile shake
            if shake {
                let (shake, flag) = self.tile_shake.entry(pos).or_insert((vec2::ZERO, true));
                *flag = true;
                let t = model.drone.action_progress.as_f32();
                *shake = *shake * 0.5
                    + Angle::from_degrees(thread_rng().gen_range(0.0..=360.0)).unit_vec()
                        * 0.05
                        * t;
                transform *= mat3::translate(*shake);
            }

            if highlighted_tiles.contains(&pos) {
                // Highlight tiles of the same type
                let t = (self.active_highlight.2.as_f32()
                    * std::f32::consts::FRAC_PI_2
                    * HOVER_BLINK_SPEED)
                    .sin()
                    * 0.5
                    + 0.5;
                mult *= 1.0 + self.active_highlight.1.as_f32() * t * HOVER_HIGHLIGHT;
            }
            let color = Color::new(mult, mult, mult, 1.0);
            self.util.draw_on_tile_with(
                &model.grid_visual,
                pos,
                color,
                transform,
                texture,
                &model.camera,
                framebuffer,
            );

            if !hide_ui
                && tile.tile.state.alive()
                && let Some(t) = tile.tile.kind.action_progress(&model.config)
            {
                // Tile action progress
                let t = t.as_f32();
                let pos = Aabb2::point(
                    model.grid_visual.tile_center(pos).as_f32() + vec2(0.0, -8.0) * pixel_scale,
                )
                .extend_symmetric(vec2(8.0, 2.0) * pixel_scale);
                self.context.geng.draw2d().quad(
                    framebuffer,
                    &model.camera,
                    pos,
                    palette.progress_background,
                );
                let progress_pos = pos.extend_uniform(-pixel_scale).split_left(t);
                self.context.geng.draw2d().quad(
                    framebuffer,
                    &model.camera,
                    progress_pos,
                    palette.progress,
                );

                if let TileKind::Seed(seed) = &tile.tile.kind
                    && seed.growth_timer < Time::ONE
                {
                    let t = 1.0 - seed.growth_timer.as_f32();
                    let progress_pos = pos
                        .extend_symmetric(-vec2(1.0, 1.0) * pixel_scale)
                        .split_left(t);
                    self.context.geng.draw2d().quad(
                        framebuffer,
                        &model.camera,
                        progress_pos,
                        crate::util::with_alpha(palette.progress.map_rgb(|x| x * 0.8), 0.8),
                    );
                }
            }
        }

        // Tile Connections
        // TODO: draw connections before tiles, but separate the shadows
        let mut connections: HashMap<(vec2<ICoord>, vec2<ICoord>), Color> = HashMap::new();
        let mut add_connection = |a: vec2<ICoord>, b: vec2<ICoord>, color: Color| {
            let a = (a.x, a.y);
            let b = (b.x, b.y);
            let (a, b) = min_max(a, b);
            let a = vec2(a.0, a.1);
            let b = vec2(b.0, b.1);
            connections.insert((a, b), color);
        };
        for tile in model.grid.all_tiles() {
            if !tile.tile.state.alive() {
                continue;
            }
            match &tile.tile.kind {
                TileKind::Leaf(leaf) => {
                    for connected_to in leaf.connections.get_connections(tile.pos) {
                        add_connection(tile.pos, connected_to.pos, palette.connection_plant);
                    }
                }
                _ if tile.tile.kind.transmits_power() => {
                    for neighbor in model.grid.get_neighbors(tile.pos) {
                        if neighbor.tile.kind.transmits_power()
                            && !matches!(neighbor.tile.state, TileState::Despawning { .. })
                        {
                            add_connection(tile.pos, neighbor.pos, palette.connection_power);
                        }
                    }
                }
                _ => {}
            }
        }
        for ((a, b), color) in connections {
            let mut pos = model
                .grid_visual
                .multitile_bounds(Aabb2::from_corners(a, b))
                .center();
            let texture = if a.x == b.x {
                pos += (vec2(0.0, 1.5) * pixel_scale).as_r32();
                &sprites.tile_connector_vertical
            } else {
                &sprites.tile_connector_horizontal
            };
            self.util.draw_texture_autoscaled(
                pos,
                Angle::ZERO,
                color,
                texture,
                &model.camera,
                framebuffer,
            );
        }

        let tile_highlight_with =
            |name: &str,
             offset: vec2<f32>,
             pos: vec2<ICoord>,
             color: Color,
             framebuffer: &mut ugli::Framebuffer<'_>| {
                self.util.draw_on_tile_with(
                    &model.grid_visual,
                    pos,
                    color,
                    mat3::translate(offset),
                    &sprites.tile_select,
                    &model.camera,
                    framebuffer,
                );
                if !name.is_empty() {
                    let tile_bounds = model.grid_visual.tile_bounds(pos).as_f32();
                    let select_size =
                        sprites.tile_select.size().as_f32() / TILE_SIZE_PIXELS.as_f32();
                    let select_bounds = tile_bounds.align_aabb(select_size, vec2(0.5, 0.5));
                    let position =
                        select_bounds.align_pos(vec2(0.0, 1.0)) + vec2(0.1, 0.0) + offset;
                    self.util.draw_text(
                        name,
                        position,
                        &assets.fonts.aseprite,
                        TextRenderOptions::new(0.3)
                            .align(vec2(0.0, 0.0))
                            .color(palette.text),
                        &model.camera,
                        framebuffer,
                    );
                }
            };
        let tile_highlight = |name: Option<&str>,
                              pos: vec2<ICoord>,
                              color: Color,
                              framebuffer: &mut ugli::Framebuffer| {
            let mut offset = vec2::ZERO;
            if let Some(tile) = model.grid.get_tile(pos)
                && let TileState::Moving { timer, delta } = &tile.tile.state
            {
                offset = movement_animation(&model.grid_visual, timer, *delta);
            }
            let name = name.or_else(|| {
                if let Some(tile) = model.grid.get_tile(pos)
                    && !matches!(tile.tile.state, TileState::Despawning { .. })
                    && !matches!(tile.tile.kind, TileKind::GhostBlock(_))
                {
                    Some(model.tile_interaction(pos).name())
                } else {
                    None
                }
            });
            if let Some(name) = name {
                tile_highlight_with(name, offset, pos, color, framebuffer);
            }
        };
        let ghost_tile = |pos: vec2<ICoord>,
                          tile: &TileKind,
                          color: Color,
                          framebuffer: &mut ugli::Framebuffer<'_>| {
            if model
                .grid
                .get_tile(pos)
                .is_none_or(|tile| matches!(tile.tile.state, TileState::Despawning { .. }))
                && let Some(texture) = sprites.tiles.get_texture(tile)
            {
                if model.grid.get_tile(pos).is_none() {
                    self.util.draw_on_tile(
                        &model.grid_visual,
                        pos,
                        Color::new(0.7, 0.7, 0.7, HOVER_ALPHA),
                        texture,
                        &model.camera,
                        framebuffer,
                    );
                }
                tile_highlight(Some(DroneAction::PlaceTile.name()), pos, color, framebuffer);
            }
        };

        // Drone and Queued actions
        for (action, alpha) in itertools::chain![
            model.all_drone_actions().map(|target| (target, 1.0)),
            model
                .all_queued_actions()
                .map(|action| (action, QUEUED_ALPHA))
        ] {
            let white = crate::util::with_alpha(Color::WHITE, alpha);
            match *action {
                DroneTarget::MoveTo(_) => {}
                DroneTarget::Collect(target) | DroneTarget::CutPlant(target) => {
                    tile_highlight(Some(action.name()), target, white, framebuffer);
                }
                DroneTarget::PlaceTile(target, ref tile)
                | DroneTarget::BuyTile(target, ref tile) => {
                    ghost_tile(target, tile, white, framebuffer);
                    tile_highlight(Some(action.name()), target, white, framebuffer);
                }
                DroneTarget::KillBug(bug_id) => {
                    let bug = model.grid.tiles.iter().find(|(_, tile)| {
                        if let TileKind::Bug(bug) = &tile.kind
                            && bug.id == bug_id
                        {
                            true
                        } else {
                            false
                        }
                    });
                    if let Some((&target, _)) = bug {
                        tile_highlight(
                            Some(action.name()),
                            target,
                            crate::util::with_alpha(Color::RED, alpha),
                            framebuffer,
                        );
                    }
                }
            }
        }

        // Input state
        if !focus_ui && let Some(target) = cursor.grid_pos {
            let tile_action = |framebuffer: &mut ugli::Framebuffer| {
                let color = if let Some(tile) = model.grid.get_tile(target)
                    && let TileKind::Bug(_) = tile.tile.kind
                {
                    Color::new(0.7, 0.1, 0.1, 0.5)
                } else {
                    Color::new(0.7, 0.7, 0.7, 0.5)
                };
                tile_highlight(None, target, color, framebuffer);
                if let Some(tile) = model.grid.get_tile(target)
                    && tile.tile.state.alive()
                {
                    let pos = model
                        .grid_visual
                        .tile_bounds(target)
                        .as_f32()
                        .align_pos(vec2(0.0, 0.0));
                    self.tile_description(
                        pos,
                        6.0,
                        0.5,
                        &tile.tile.kind,
                        model,
                        false,
                        pixel_scale,
                        &model.camera,
                        framebuffer,
                    );
                }
            };
            match input_state {
                InputState::Idle => tile_action(framebuffer),
                _ if model.grid.get_tile(target).is_some_and(|tile| {
                    !matches!(tile.tile.state, TileState::Despawning { .. })
                }) =>
                {
                    tile_action(framebuffer)
                }
                InputState::PlaceTile(tile) | InputState::BuyTile(tile) => {
                    ghost_tile(target, tile, Color::new(0.7, 0.7, 0.7, 0.5), framebuffer);
                }
            }
        }

        // Drone
        let angle = Angle::from_radians(
            model.drone.velocity.x.signum()
                * model.drone.velocity.y.signum()
                * model.drone.velocity.len()
                / assets.config.drone_max_speed
                * r32(0.5),
        );
        self.util.draw_texture_autoscaled(
            model.drone.position,
            angle.as_f32(),
            Color::WHITE,
            &sprites.drone,
            &model.camera,
            framebuffer,
        );
        if !hide_ui && model.drone.action_progress > R32::ZERO {
            // Drone progress
            let t = model.drone.action_progress.as_f32();
            let pos = Aabb2::point(model.drone.position.as_f32() + vec2(0.0, -10.0) * pixel_scale)
                .extend_symmetric(vec2(8.0, 2.0) * pixel_scale);
            self.context.geng.draw2d().quad(
                framebuffer,
                &model.camera,
                pos,
                palette.progress_background,
            );
            let pos = pos.extend_uniform(-pixel_scale).split_left(t);
            self.context
                .geng
                .draw2d()
                .quad(framebuffer, &model.camera, pos, palette.progress);
        }
    }

    pub fn draw_ui(
        &mut self,
        ui: &GameUI,
        model: &Model,
        input_state: &InputState,
        ui_context: &UiContext,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let assets = self.context.assets.clone();
        let sprites = &assets.sprites;
        let palette = &assets.palette;

        let pixel_scale = get_pixel_scale(framebuffer.size());
        let font_size = 12.0 * pixel_scale;

        // Inventory
        self.util.draw_nine_slice(
            ui.inventory.position,
            Color::WHITE,
            &sprites.ui_window,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Inventory items
        for (widget, (tile, count)) in ui.inventory_items.iter().zip(&model.inventory) {
            let Some(texture) = &sprites.tiles.get_texture(tile) else {
                // TODO: placeholder texture
                continue;
            };

            let mut scale = vec2(1.0, 1.0);
            if let Some(t) = widget.hovered_time {
                scale = hover_animation(t / HOVER_ANIMATION_TIME);
            }
            self.ui.draw_texture_with(
                widget.position,
                texture,
                Color::WHITE,
                1.0,
                mat3::scale(scale),
                framebuffer,
            );

            // Count
            let pos = widget.position.align_pos(vec2(0.5, 1.0)) + vec2(0.0, 3.0) * pixel_scale;
            self.util.draw_text(
                count.to_string(),
                pos,
                &self.context.assets.fonts.aseprite,
                TextRenderOptions::new(font_size)
                    .color(palette.text)
                    .align(vec2(0.5, 0.0)),
                &geng::PixelPerfectCamera,
                framebuffer,
            );
        }

        // Shop
        self.util.draw_nine_slice(
            ui.shop.position,
            Color::WHITE,
            &sprites.ui_window,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Shop items
        for (widget, tile) in &ui.shop_items {
            let unlock_cost = model
                .config
                .shop
                .iter()
                .find(|item| {
                    item.tile == *tile
                        && item.unlocked_at > 0
                        && !model.unlocked_shop.contains(tile)
                })
                .map(|item| item.unlocked_at);

            let Some(texture) = &sprites.tiles.get_texture(tile) else {
                // TODO: placeholder texture
                continue;
            };
            let m = if unlock_cost.is_some() {
                SHOP_TILE_LOCKED
            } else if model.config.get_cost(tile) > model.money {
                SHOP_TILE_TOO_EXPENSIVE
            } else {
                1.0
            };
            let color = Color::new(m, m, m, 1.0);

            let mut scale = vec2(1.0, 1.0);
            if let Some(t) = widget.hovered_time {
                scale = hover_animation(t / HOVER_ANIMATION_TIME);
            }
            self.ui.draw_texture_with(
                widget.position,
                texture,
                color,
                1.0,
                mat3::scale(scale),
                framebuffer,
            );

            // Cost
            let (cost, pos) = match unlock_cost {
                Some(unlock) => (unlock, widget.position.center()),
                None => (
                    model.config.get_cost(tile),
                    widget.position.align_pos(vec2(0.5, 1.0)) + vec2(0.0, 3.0) * pixel_scale,
                ),
            };
            self.util.draw_text_gold(
                cost,
                pos,
                &self.context.assets.fonts.aseprite,
                TextRenderOptions::new(font_size)
                    .color(palette.text)
                    .align(vec2(0.5, 0.0)),
                &geng::PixelPerfectCamera,
                framebuffer,
            );
        }

        // Gold
        self.util.draw_nine_slice(
            ui.gold.position,
            Color::WHITE,
            &sprites.ui_window,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );
        let pos = ui.gold.position.extend_uniform(-0.0 * pixel_scale);
        self.util.draw_text_gold(
            model.money,
            pos.center(),
            &self.context.assets.fonts.aseprite,
            TextRenderOptions::new(font_size)
                .color(palette.text)
                .align(vec2(0.5, 0.5)),
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Hovered tile description
        for (widget, tile) in &ui.shop_items {
            if widget.hovered {
                self.tile_description(
                    widget.position.bottom_left(),
                    5.0 * pixel_scale * TILE_SIZE_PIXELS.y as f32,
                    0.4 * pixel_scale * TILE_SIZE_PIXELS.y as f32,
                    tile,
                    model,
                    true,
                    pixel_scale,
                    &geng::PixelPerfectCamera,
                    framebuffer,
                );
            }
        }
        for (widget, (tile, _)) in ui.inventory_items.iter().zip(&model.inventory) {
            if widget.hovered {
                self.tile_description(
                    widget.position.top_right() + vec2(5.0, 0.0) * pixel_scale,
                    5.0 * pixel_scale * TILE_SIZE_PIXELS.y as f32,
                    0.4 * pixel_scale * TILE_SIZE_PIXELS.y as f32,
                    tile,
                    model,
                    true,
                    pixel_scale,
                    &geng::PixelPerfectCamera,
                    framebuffer,
                );
            }
        }

        // Placement ghost
        if let InputState::PlaceTile(kind) | InputState::BuyTile(kind) = input_state
            && (ui.inventory.hovered || ui.shop.hovered)
            && let Some(texture) = sprites.tiles.get_texture(kind)
        {
            let color = Color::new(0.7, 0.7, 0.7, HOVER_ALPHA);
            let size = vec2(28.0, 28.0) * pixel_scale;
            let quad = Aabb2::point(ui_context.cursor.position).extend_symmetric(size / 2.0);
            self.context.geng.draw2d().draw2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw2d::TexturedQuad::colored(quad, &**texture, color),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn tile_description(
        &self,
        pos: vec2<f32>,
        width: f32,
        font_size: f32,
        tile: &TileKind,
        model: &Model,
        clamp_screen: bool,
        pixel_scale: f32,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer<'_>,
    ) {
        let assets = &self.context.assets;

        if let TileKind::GhostBlock(_) = tile {
            return;
        }

        let description = if !model.unlocked_shop.contains(tile)
            && model
                .config
                .shop
                .iter()
                .any(|shop| shop.tile == *tile && shop.unlocked_at > 0)
        {
            "Locked"
        } else {
            tile.description()
        };

        let text = format!("{}\n-----\n{}", tile.name(), description);

        let lines = crate::util::wrap_text(
            &self.context.assets.fonts.aseprite,
            &text,
            width / font_size,
        );

        let height = font_size * 0.75 + font_size * lines.len() as f32;

        let mut pos = Aabb2::point(pos).extend_right(width).extend_down(height);

        if clamp_screen {
            // Clamp by screen bounds
            let screen_margin = pixel_scale * 5.0;
            let screen = Aabb2::ZERO
                .extend_positive(framebuffer.size().as_f32())
                .extend_uniform(-screen_margin);
            let screen_pos = pos.map_bounds(|pos| {
                match camera.world_to_screen(framebuffer.size().as_f32(), pos) {
                    Ok(p) | Err(p) => p,
                }
            });
            let mut offset = vec2::ZERO;
            if screen_pos.max.x > screen.max.x {
                offset.x = screen.max.x - screen_pos.max.x;
            }
            if screen_pos.min.y < screen.min.y {
                offset.y = screen.min.y - screen_pos.min.y;
            }
            let target =
                camera.screen_to_world(framebuffer.size().as_f32(), screen_pos.center() + offset);
            pos = pos.translate(target - pos.center());
        }

        self.util.draw_nine_slice(
            pos,
            Color::new(1.0, 1.0, 1.0, 0.8),
            &assets.sprites.ui_window,
            pixel_scale,
            camera,
            framebuffer,
        );

        let pos = pos.extend_uniform(-3.0 * pixel_scale);
        let row = pos.align_aabb(vec2(pos.width(), font_size), vec2(0.5, 1.0));
        let rows = row.stack(vec2(0.0, -row.height()), lines.len());

        for (line, position) in lines.into_iter().zip(rows) {
            self.util.draw_text(
                line,
                position.align_pos(vec2(0.0, 0.5)),
                &self.context.assets.fonts.aseprite,
                TextRenderOptions::new(font_size)
                    .color(crate::util::with_alpha(assets.palette.text, 1.0))
                    .align(vec2(0.0, 0.5)),
                camera,
                framebuffer,
            );
        }
    }
}

fn hover_animation(t: f32) -> vec2<f32> {
    let t = t.clamp(0.0, 1.0);
    let t = 1.0 - crate::util::ease_out_elastic_with(t, 3.0, 1.0);
    let stretch = 1.0 + 0.3 * t;
    let squish = 1.0 - 0.3 * t;
    vec2(squish, stretch)
}

fn movement_animation(grid: &GridVisual, timer: &Lifetime, delta: vec2<ICoord>) -> vec2<f32> {
    let t = timer.ratio().as_f32();
    let t = crate::util::smoothstep(1.0 - t);
    grid.tile_size.as_f32() * delta.as_f32() * t
}
