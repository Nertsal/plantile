mod actions;
mod drone;
mod plants;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: Time) {
        self.update_drone_position(delta_time);
    }

    pub fn fixed_update(&mut self, delta_time: Time) {
        self.update_action_queue();
        self.update_drone(delta_time);

        // Update tiles
        let update_order: Vec<vec2<ICoord>> = self
            .grid
            .all_tiles()
            .sorted_by_key(|tile| tile.tile.kind.update_order())
            .map(|tile| tile.pos)
            .collect();
        for &pos in &update_order {
            self.update_tile_state(pos, delta_time);
        }
        for pos in update_order {
            self.tile_logic(pos, delta_time);
        }

        self.rng_spawn(delta_time);
    }

    fn update_tile_state(&mut self, pos: vec2<ICoord>, delta_time: Time) {
        let Some(tile) = self.grid.get_tile_mut(pos) else {
            return;
        };

        match &mut tile.tile.state {
            TileState::Spawning(timer) | TileState::Transforming(timer) => {
                timer.change(-delta_time / self.config.animations.tile_spawn);
                if timer.remaining <= Time::ZERO {
                    tile.tile.state = TileState::Idle;
                }
            }
            TileState::Moving { timer, delta } => {
                timer.change(-delta_time / self.config.animations.bug_move);
                let delta = *delta;
                if timer.remaining <= Time::ZERO
                    && let Some(mut tile) = self.grid.remove_tile(pos)
                {
                    tile.tile.state = TileState::Idle;
                    self.grid.set_tile(pos + delta, tile.tile);
                }
            }
            TileState::Idle => {}
            TileState::Despawning(timer) => {
                timer.change(-delta_time / self.config.animations.tile_despawn);
                if timer.remaining <= Time::ZERO {
                    self.grid.remove_tile(pos);
                }
            }
        }
    }

    fn tile_logic(&mut self, pos: vec2<ICoord>, delta_time: Time) {
        let Some(tile) = self.grid.get_tile_mut(pos) else {
            return;
        };
        if !tile.tile.state.interactive() {
            return;
        }
        let mut rng = thread_rng();

        match tile.tile.kind {
            TileKind::GhostBlock(ref reason) => {
                let justified = match reason {
                    &ExistentialReason::MoveFrom(reason_pos) => {
                        self.grid.get_tile(reason_pos).is_some()
                    }
                };
                if !justified {
                    // Ghost's existence is not justified - perish!
                    self.grid.remove_tile(pos);
                }
            }
            TileKind::Leaf(_) => self.update_plant(pos, delta_time),
            TileKind::Power => {}
            TileKind::Light(_) | TileKind::Wire(_) => {
                let mut powered = false;
                get_all_connected(&self.grid, pos, |tile| {
                    if tile.tile.state.alive()
                        && let TileKind::Power = tile.tile.kind
                    {
                        powered = true;
                    }
                    tile.tile.state.alive() && tile.tile.kind.transmits_power()
                });
                if let Some(tile) = self.grid.get_tile_mut(pos)
                    && let TileKind::Light(power) | TileKind::Wire(power) = &mut tile.tile.kind
                {
                    *power = powered;
                }
            }
            TileKind::Seed(ref seed) => {
                let plant_kind = seed.kind;
                let config = &self.config.plants[&plant_kind];
                let grow_direction = seed_grow_direction(self.config.seed_grow_only_up);

                // Current energy of the seed
                let seed_energy = seed.total_energy();

                // Grow from Soil
                let grow_from = grow_direction
                    .iter()
                    .filter_map(|delta| self.grid.get_tile(pos - *delta))
                    .filter(|tile| tile.tile.state.interactive())
                    .find_map(|neighbor| {
                        let mut kind = neighbor.tile.kind.clone();
                        if let TileKind::Water(lifetime) = &mut kind {
                            *lifetime = Lifetime::default();
                        }
                        config.soils.get(&kind).map(|config| (neighbor, config))
                    });

                if let Some((grow_from, soil_config)) = grow_from
                    && config.growth_capacity - seed_energy >= soil_config.capacity
                    && let Some(grow_from) = self.grid.get_tile_mut(grow_from.pos)
                {
                    // Take energy from soil
                    match &mut grow_from.tile.kind {
                        TileKind::Soil(state) => {
                            *state = SoilState::Dry;
                            grow_from.tile.state.transform();
                        }
                        TileKind::Water(_) => {
                            grow_from.tile.state.despawn();
                        }
                        _ => {}
                    }
                    if let Some(tile) = self.grid.get_tile_mut(pos)
                        && let TileKind::Seed(seed) = &mut tile.tile.kind
                    {
                        tile.tile.state.transform();
                        *seed
                            .growth_energy
                            .entry(soil_config.growth_speed)
                            .or_insert(Time::ZERO) += soil_config.capacity;
                    }
                } else if seed_energy >= R32::ONE
                    && !grow_direction.iter().any(|delta| {
                        if let Some(tile) = self.grid.get_tile(pos + *delta)
                            && let TileKind::Leaf(leaf) = &tile.tile.kind
                        {
                            leaf.kind == plant_kind
                        } else {
                            false
                        }
                    })
                    && let Some(empty) = grow_direction
                        .iter()
                        .map(|delta| pos + *delta)
                        .filter(|&pos| plants::can_grow_into(pos, &self.grid))
                        .choose(&mut rng)
                {
                    // Grow into a plant
                    let growth_time = if self.grid.is_tile_lit(pos, &self.config) {
                        config.growth_time
                    } else {
                        config.growth_time_dark
                    };
                    if let Some(tile) = self.grid.get_tile_mut(pos)
                        && let TileKind::Seed(seed) = &mut tile.tile.kind
                    {
                        seed.growth_timer -= delta_time / growth_time;
                        seed.growth_timer = seed.growth_timer.clamp(Time::ZERO, Time::ONE);
                        if seed.growth_timer <= Time::ZERO {
                            tile.tile.state.transform();
                            seed.use_energy(R32::ONE);
                            self.grid.set_tile(
                                empty,
                                Tile::new(TileKind::Leaf(
                                    Leaf::new(plant_kind).connected(pos - empty),
                                )),
                            );
                        }
                    }
                } else if let Some(tile) = self.grid.get_tile_mut(pos)
                    && let TileKind::Seed(seed) = &mut tile.tile.kind
                {
                    seed.growth_timer = Time::ONE;
                }
            }
            TileKind::Soil(state) => match state {
                SoilState::Dry => {
                    let water = self
                        .grid
                        .get_neighbors(pos)
                        .find(|tile| {
                            tile.tile.state.interactive()
                                && matches!(tile.tile.kind, TileKind::Water(_))
                        })
                        .map(|tile| tile.pos);
                    if let Some(water) = water
                        && let Some(water) = self.grid.get_tile_mut(water)
                    {
                        water.tile.state.despawn();
                        let soil = self.grid.get_tile_mut(pos).unwrap();
                        if let TileKind::Soil(state) = &mut soil.tile.kind {
                            *state = SoilState::Watered;
                            soil.tile.state.transform();
                        }
                    }
                }
                SoilState::Watered => {
                    let poop = self
                        .grid
                        .get_neighbors(pos)
                        .find(|tile| matches!(tile.tile.kind, TileKind::Poop(_)))
                        .map(|tile| tile.pos);
                    if let Some(poop) = poop
                        && let Some(poop) = self.grid.get_tile_mut(poop)
                    {
                        poop.tile.state.despawn();
                        let soil = self.grid.get_tile_mut(pos).unwrap();
                        if let TileKind::Soil(state) = &mut soil.tile.kind {
                            *state = SoilState::Rich;
                            soil.tile.state.transform();
                        }
                    }
                }
                SoilState::Rich => {}
            },
            TileKind::Water(_) => {
                let sprinkler = self
                    .grid
                    .get_neighbors(pos)
                    .any(|tile| matches!(tile.tile.kind, TileKind::Sprinkler(_)));
                if let Some(tile) = self.grid.get_tile_mut(pos)
                    && let TileKind::Water(lifetime) = &mut tile.tile.kind
                {
                    if sprinkler {
                        // Does not evaporate when next to sprinkler
                        lifetime.remaining = lifetime.max;
                    } else {
                        lifetime.change(-delta_time);
                        if lifetime.remaining <= Time::ZERO {
                            // Evaporate
                            tile.tile.state.despawn();
                        }
                    }
                }
            }
            TileKind::Bug(ref mut bug) => {
                if bug.move_timer > Time::ZERO {
                    if let Some(DroneTarget::KillBug(bug_id)) = self.drone.target
                        && bug.id == bug_id
                        && self.drone.action_progress > R32::ZERO
                    {
                        // Targetted by a drone - cannot move
                    } else {
                        bug.move_timer -= delta_time;
                    }
                }
                let can_move = bug.move_timer <= Time::ZERO;

                let move_towards = |target: vec2<ICoord>, grid: &mut Grid| {
                    if !can_move {
                        return;
                    }

                    let delta = target - pos;
                    let dir = if delta.x.abs() > delta.y.abs() {
                        vec2(delta.x.signum(), 0)
                    } else {
                        vec2(0, delta.y.signum())
                    };

                    if bug_can_move_into_pos(grid, tile.pos + dir)
                        && let Some(tile) = grid.get_tile_mut(pos)
                        && let TileKind::Bug(bug) = &mut tile.tile.kind
                    {
                        bug.move_timer = self.config.bug_move_time;
                        match &mut bug.state {
                            BugState::Hungry { eating_timer, .. } => {
                                eating_timer.remaining = eating_timer.max
                            }
                            BugState::Pooping(lifetime) => lifetime.remaining = lifetime.max,
                            BugState::Chilling { .. } => {}
                        }
                        tile.tile.state.moving(dir);
                        grid.set_tile(
                            pos + dir,
                            Tile::new(TileKind::GhostBlock(ExistentialReason::MoveFrom(pos))),
                        );
                    }
                };

                match &mut bug.state {
                    BugState::Hungry { hunger, .. } => {
                        if *hunger == 0 {
                            bug.state = BugState::Pooping(Lifetime::new(self.config.bug_poop_time));
                            return;
                        }

                        // Look for leaves nearby
                        let leaf_target = self
                            .grid
                            .all_tiles()
                            .filter(|tile| {
                                if manhattan_distance(pos, tile.pos)
                                    <= self.config.bug_vision_radius
                                    && tile.tile.state.interactive()
                                    && let TileKind::Leaf(_) = &tile.tile.kind
                                {
                                    true
                                } else {
                                    false
                                }
                            })
                            .min_by_key(|tile| manhattan_distance(pos, tile.pos))
                            .map(|tile| tile.pos);

                        // Go towards target
                        if let Some(target) = leaf_target {
                            // Go to the leaves
                            if manhattan_distance(pos, target) <= 1
                                && let Some(tile) = self.grid.get_tile(target)
                                && let TileKind::Leaf(_) = tile.tile.kind
                            {
                                // eat
                                if let Some(bug) = self.grid.get_tile_mut(pos)
                                    && let TileKind::Bug(bug) = &mut bug.tile.kind
                                    && let BugState::Hungry {
                                        eating_timer,
                                        hunger,
                                    } = &mut bug.state
                                {
                                    eating_timer.change(-delta_time);
                                    if eating_timer.remaining <= Time::ZERO {
                                        *eating_timer = Lifetime::new(self.config.bug_eat_time);
                                        *hunger -= 1;
                                        self.cut_plant_tile(target, false);
                                        self.context.sfx.play(&self.context.assets.sounds.bug_eat);
                                    }
                                }
                            } else {
                                // move towards the plant
                                if let Some(path) = self.grid.bug_find_path(pos, target)
                                    && let Some(&next) = path.get(1)
                                {
                                    move_towards(next, &mut self.grid);
                                }
                            }
                        } else {
                            // move in available random direction
                            if let Some(target) = self
                                .grid
                                .get_neighbors_all(pos)
                                .filter(|tile| bug_can_move_into_pos(&self.grid, tile.pos))
                                .map(|tile| tile.pos)
                                .choose(&mut rng)
                            {
                                move_towards(target, &mut self.grid);
                            }
                        }
                    }
                    BugState::Pooping(timer) => {
                        timer.change(-delta_time);
                        if timer.remaining <= Time::ZERO {
                            let target = self
                                .grid
                                .get_neighbors_all(pos)
                                .find(|tile| tile.tile.is_none())
                                .map(|tile| tile.pos);
                            if let Some(target) = target {
                                self.grid.set_tile(
                                    target,
                                    Tile::new(TileKind::Poop(Lifetime::new(
                                        self.config.poop_lifetime,
                                    ))),
                                );
                                self.context.sfx.play(&self.context.assets.sounds.bug_poop);
                                if let Some(bug) = self.grid.get_tile_mut(pos)
                                    && let TileKind::Bug(bug) = &mut bug.tile.kind
                                {
                                    bug.state = BugState::Chilling {
                                        time: self.config.bug_chill_time,
                                    }
                                }
                            }
                        }
                    }
                    BugState::Chilling { time } => {
                        *time -= delta_time;
                        if *time <= Time::ZERO {
                            bug.state = BugState::Hungry {
                                hunger: self.config.bug_hunger,
                                eating_timer: Lifetime::new(self.config.bug_eat_time),
                            };
                        } else {
                            // Move in available random direction
                            if let Some(target) = self
                                .grid
                                .get_neighbors_all(pos)
                                .filter(|tile| bug_can_move_into_pos(&self.grid, tile.pos))
                                .map(|tile| tile.pos)
                                .choose(&mut rng)
                            {
                                move_towards(target, &mut self.grid);
                            }
                        }
                    }
                }
            }
            TileKind::Poop(ref mut lifetime) => {
                lifetime.change(-delta_time);
                if lifetime.remaining <= Time::ZERO {
                    tile.tile.state.despawn();
                }
            }
            TileKind::Drainer => {
                let water = self
                    .grid
                    .all_tiles()
                    .find(|tile| {
                        // Collect water within range not adjacent to a sprinkler
                        tile.tile.state.interactive()
                            && matches!(tile.tile.kind, TileKind::Water(_))
                            && manhattan_distance(pos, tile.pos) <= self.config.drainer_radius
                            && !self
                                .grid
                                .get_neighbors(tile.pos)
                                .any(|tile| matches!(tile.tile.kind, TileKind::Sprinkler(_)))
                    })
                    .map(|tile| tile.pos);
                if let Some(water) = water {
                    // Look for a sprinkler
                    let mut sprinklers = Vec::new();
                    get_all_connected(&self.grid, pos, |tile| {
                        if tile.tile.state.alive()
                            && let TileKind::Sprinkler(_) = tile.tile.kind
                        {
                            sprinklers.push(tile.pos);
                        }
                        tile.tile.state.alive() && tile.tile.kind.is_piping()
                    });

                    let empty_tiles: HashMap<vec2<ICoord>, vec2<ICoord>> = sprinklers
                        .into_iter()
                        .flat_map(|sprinkler_pos| {
                            self.grid
                                .get_neighbors_all(sprinkler_pos)
                                .filter(|tile| tile.tile.is_none())
                                .map(move |tile| (tile.pos, sprinkler_pos))
                        })
                        .collect();
                    if let Some((target, sprinkler_pos)) = empty_tiles.into_iter().choose(&mut rng)
                    {
                        // Pipe water to a sprinkler
                        if let Some(water) = self.grid.get_tile_mut(water) {
                            water.tile.state.despawn();
                        }
                        self.grid.set_tile(
                            target,
                            Tile::new(TileKind::Water(Lifetime::new(self.config.water_lifetime))),
                        );
                        if let Some(sprinkler) = self.grid.get_tile_mut(sprinkler_pos) {
                            sprinkler.tile.state.transform()
                        }
                    } else {
                        // Collect water to player inventory
                        self.collect(water);
                    }
                }
            }
            TileKind::Cutter(_) => {
                let mut powered = false;
                get_all_connected(&self.grid, pos, |tile| {
                    if tile.tile.state.alive()
                        && let TileKind::Power = tile.tile.kind
                    {
                        powered = true;
                    }
                    tile.tile.state.alive() && tile.tile.kind.transmits_power()
                });
                let plants: Vec<_> = self
                    .grid
                    .get_neighbors(pos)
                    .filter(|tile| {
                        matches!(tile.tile.kind, TileKind::Leaf(_))
                            && tile.tile.state.alive()
                            && get_whole_plant(&self.grid, tile.pos, &self.config)
                                .iter()
                                .all(|&pos| {
                                    if let Some(tile) = self.grid.get_tile(pos)
                                        && let TileKind::Leaf(leaf) = &tile.tile.kind
                                    {
                                        !leaf.is_growing
                                    } else {
                                        true
                                    }
                                })
                    })
                    .map(|tile| tile.pos)
                    .collect();

                if let Some(tile) = self.grid.get_tile_mut(pos)
                    && let TileKind::Cutter(cutter) = &mut tile.tile.kind
                {
                    cutter.powered = powered;
                    if powered && !plants.is_empty() {
                        cutter
                            .cut_timer
                            .change(-delta_time / self.config.cutter_cut_time);
                        if cutter.cut_timer.remaining <= Time::ZERO {
                            // Cut down a nearby plant
                            cutter.cut_timer.remaining = cutter.cut_timer.max;
                            for plant in plants {
                                self.cut_plant_tile(plant, true);
                            }
                        }
                    } else {
                        cutter.cut_timer.remaining = cutter.cut_timer.max;
                    }
                }
            }
            TileKind::Pipe(_) | TileKind::Sprinkler(_) => {
                let mut piped = false;
                get_all_connected(&self.grid, pos, |tile| {
                    if tile.tile.state.alive()
                        && let TileKind::Drainer = tile.tile.kind
                    {
                        piped = true;
                    }
                    tile.tile.state.alive() && tile.tile.kind.is_piping()
                });
                if let Some(tile) = self.grid.get_tile_mut(pos)
                    && let TileKind::Pipe(connected) | TileKind::Sprinkler(connected) =
                        &mut tile.tile.kind
                {
                    *connected = piped;
                }
            }
            TileKind::Rock => {}
        }
    }

    /// Cut plant from seed or leaf.
    fn cut_plant_tile(&mut self, target: vec2<ICoord>, earn_money: bool) {
        let Some(tile) = self.grid.get_tile(target) else {
            return;
        };
        let (plant_kind, leaf_connections) = match &tile.tile.kind {
            TileKind::Leaf(leaf) => (leaf.kind, Some(leaf.connections.clone())),
            TileKind::Seed(seed) => (seed.kind, None),
            _ => return,
        };
        let config = &self.config.plants[&plant_kind];
        let despawn = if leaf_connections.is_some() {
            if earn_money {
                self.money += config.price;
            }
            true
        } else {
            let seed = TileKind::Seed(Seed::new(plant_kind));
            if self.can_collect(&seed) {
                self.inventory_add(seed, 1);
                true
            } else {
                false
            }
        };

        let Some(tile) = self.grid.get_tile_mut(target) else {
            return;
        };
        if despawn {
            tile.tile.state.despawn();
        }

        let mut lost_plants = Vec::new();
        for tile in self.grid.get_neighbors(target) {
            if lost_plants.contains(&tile.pos) {
                continue;
            }
            if let TileKind::Leaf(leaf) = &tile.tile.kind
                && leaf.kind == plant_kind
            {
                // Check connectivity to root
                let group = get_whole_plant(&self.grid, tile.pos, &self.config);
                let rooted = group.iter().any(|&pos| {
                    if let Some(tile) = self.grid.get_tile(pos)
                        && tile.tile.state.alive()
                        && let TileKind::Seed(seed) = &tile.tile.kind
                    {
                        seed.kind == plant_kind
                    } else {
                        false
                    }
                });
                if !rooted {
                    lost_plants.extend(group);
                }
            }
        }

        for tile in lost_plants {
            if let Some(tile) = self.grid.get_tile_mut(tile) {
                tile.tile.state.despawn();
                if earn_money && let TileKind::Leaf(leaf) = &tile.tile.kind {
                    let config = &self.config.plants[&leaf.kind];
                    self.money += config.price;
                }
            }
        }
    }

    fn rng_spawn(&mut self, delta_time: Time) {
        let mut rng = thread_rng();

        // Rock
        let chance = self.config.rock_frequency * delta_time;
        if rng.gen_bool(chance.as_f32().into()) {
            // attempt to spawn
            for _ in 0..5 {
                let bounds = self.grid.bounds;
                let pos = vec2(
                    rng.gen_range(bounds.min.x..=bounds.max.x),
                    rng.gen_range(bounds.min.y..=bounds.max.y),
                );
                if self.grid.get_tile(pos).is_none() {
                    self.grid.set_tile(pos, Tile::new(TileKind::Rock));
                    break;
                }
            }
        }

        // Water
        {
            let anchors: Vec<_> = self
                .grid
                .all_positions()
                .filter(|pos| {
                    self.grid.get_tile(*pos).is_some_and(|tile| {
                        if let TileKind::Leaf(leaf) = &tile.tile.kind {
                            leaf.growth_timer.is_some()
                        } else {
                            false
                        }
                    })
                })
                .collect();
            let chance = (self.config.water_frequency * delta_time).as_f32() * anchors.len() as f32;
            if rng.gen_bool(chance.clamp(0.0, 1.0).into()) {
                // attempt to spawn
                let range = 2;
                let candidates = anchors
                    .into_iter()
                    .flat_map(|pos| {
                        (-range..=range)
                            .flat_map(move |dx| (-range..=range).map(move |dy| pos + vec2(dx, dy)))
                    })
                    .filter(|&pos| self.grid.in_bounds(pos) && self.grid.get_tile(pos).is_none());
                if let Some(target) = candidates.choose(&mut rng) {
                    self.grid.set_tile(
                        target,
                        Tile::new(TileKind::Water(Lifetime::new(self.config.water_lifetime))),
                    );
                }
            }
        }

        // Bug
        let total_bugs = self
            .grid
            .all_tiles()
            .filter(|tile| matches!(tile.tile.kind, TileKind::Bug(_)))
            .count();
        if total_bugs <= self.config.bug_population {
            let chance = self.config.bug_frequency * delta_time;
            if rng.gen_bool(chance.as_f32().into()) {
                // attempt to spawn
                for _ in 0..10 {
                    let bounds = self.grid.bounds;
                    let pos = vec2(
                        rng.gen_range(bounds.min.x..=bounds.max.x),
                        rng.gen_range(bounds.min.y..=bounds.max.y),
                    );
                    if self.grid.get_tile(pos).is_none()
                        && !self.grid.is_tile_lit(pos, &self.config)
                    {
                        self.grid.set_tile(
                            pos,
                            Tile::new(TileKind::Bug(Bug {
                                id: self.next_id,
                                state: BugState::Hungry {
                                    hunger: self.config.bug_hunger,
                                    eating_timer: Lifetime::new(self.config.bug_eat_time),
                                },
                                move_timer: self.config.bug_move_time,
                            })),
                        );
                        self.next_id += 1;
                        break;
                    }
                }
            }
        }
    }

    pub fn active_action_at(&mut self, target: vec2<ICoord>) -> Option<(ActionId, &DroneTarget)> {
        let mut actions = itertools::chain![
            self.drone
                .target
                .iter()
                .map(|action| (ActionId::Drone, action)),
            self.queued_actions
                .iter()
                .enumerate()
                .map(|(i, action)| (ActionId::Queued(i), action))
        ];

        actions.find(|(_, action)| match **action {
            DroneTarget::Collect(pos)
            | DroneTarget::CutPlant(pos)
            | DroneTarget::PlaceTile(pos, _)
            | DroneTarget::BuyTile(pos, _) => pos == target,
            DroneTarget::KillBug(id) => self.grid.get_tile(target).is_some_and(|tile| {
                if let TileKind::Bug(bug) = &tile.tile.kind {
                    bug.id == id
                } else {
                    false
                }
            }),
            DroneTarget::MoveTo(_) => false,
        })
    }
}

impl Grid {
    pub fn bug_find_path(
        &self,
        from: vec2<ICoord>,
        plant: vec2<ICoord>,
    ) -> Option<Vec<vec2<ICoord>>> {
        pathfinding::directed::astar::astar(
            &from,
            |&pos| {
                self.get_neighbors_all(pos)
                    .filter(|tile| match tile.tile {
                        None => true,
                        Some(tile) => bug_can_move_into(&tile.kind),
                    })
                    .map(|tile| (tile.pos, 1))
            },
            |&from| manhattan_distance(from, plant),
            |&pos| {
                self.get_neighbors(pos)
                    .any(|tile| matches!(tile.tile.kind, TileKind::Leaf(_)))
            },
        )
        .map(|(p, _)| p)
    }
}

pub fn get_whole_plant(grid: &Grid, start: vec2<ICoord>, config: &Config) -> Vec<vec2<ICoord>> {
    let mut connected = vec![start];
    let mut to_check = vec![start];
    while let Some(pos) = to_check.pop() {
        if let Some(tile) = grid.get_tile(pos)
            && tile.tile.state.alive()
        {
            let (plant_kind, connections) = match &tile.tile.kind {
                TileKind::Leaf(leaf) => (
                    leaf.kind,
                    leaf.connections
                        .get_connections(tile.pos)
                        .map(|other| other.pos)
                        .collect::<Vec<_>>(),
                ),
                TileKind::Seed(seed) => (
                    seed.kind,
                    seed_grow_direction(config.seed_grow_only_up)
                        .into_iter()
                        .map(|delta| pos + delta)
                        .collect(),
                ),
                _ => continue,
            };
            let connections: Vec<_> = connections
                .into_iter()
                .filter(|&other| {
                    !connected.contains(&other)
                        && grid
                            .get_tile(other)
                            .is_some_and(|other| match &other.tile.kind {
                                TileKind::Leaf(other) => other.kind == plant_kind,
                                TileKind::Seed(seed) => seed.kind == plant_kind,
                                _ => false,
                            })
                })
                .collect();
            connected.extend(connections.clone());
            to_check.extend(connections.clone());
        }
    }
    connected
}

fn get_all_connected(
    grid: &Grid,
    start: vec2<ICoord>,
    mut condition: impl FnMut(Positioned<&Tile>) -> bool,
) -> Vec<vec2<ICoord>> {
    let mut connected = vec![start];
    let mut to_check = vec![start];

    while let Some(pos) = to_check.pop() {
        for tile in grid.get_neighbors(pos) {
            if !connected.contains(&tile.pos) && condition(tile) {
                connected.push(tile.pos);
                to_check.push(tile.pos);
            }
        }
    }

    connected
}

// pub fn aabb_contains(aabb: Aabb2<ICoord>, pos: vec2<ICoord>) -> bool {
//     aabb.min.x <= pos.x && aabb.min.y <= pos.y && aabb.max.x >= pos.x && aabb.max.y >= pos.y
// }

pub fn manhattan_distance(a: vec2<ICoord>, b: vec2<ICoord>) -> ICoord {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn bug_can_move_into_pos(grid: &Grid, pos: vec2<ICoord>) -> bool {
    grid.get_tile(pos)
        .is_none_or(|tile| bug_can_move_into(&tile.tile.kind))
}

fn bug_can_move_into(tile: &TileKind) -> bool {
    matches!(tile, TileKind::Wire(_) | TileKind::Pipe(_))
}

fn seed_grow_direction(only_up: bool) -> Vec<vec2<ICoord>> {
    if only_up {
        vec![vec2(0, 1)]
    } else {
        Connections::NEIGHBORS.to_vec()
    }
}
