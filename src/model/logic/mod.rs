mod actions;
mod plants;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: Time) {
        let mut rng = thread_rng();

        self.update_drone(delta_time);

        // Update tiles
        let update_order: Vec<vec2<ICoord>> = self
            .grid
            .all_tiles()
            .sorted_by_key(|tile| tile.tile.kind.update_order())
            .map(|tile| tile.pos)
            .collect();
        for pos in update_order {
            let Some(tile) = self.grid.get_tile_mut(pos) else {
                continue;
            };

            match &mut tile.tile.state {
                TileState::Spawning(timer) | TileState::Transforming(timer) => {
                    timer.change(-delta_time / self.config.animations.tile_spawn);
                    if timer.remaining <= Time::ZERO {
                        tile.tile.state = TileState::Idle;
                    }
                    continue;
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
                    continue;
                }
                TileState::Idle => {}
                TileState::Despawning(timer) => {
                    timer.change(-delta_time / self.config.animations.tile_despawn);
                    if timer.remaining <= Time::ZERO {
                        self.grid.remove_tile(pos);
                    }
                    continue;
                }
            }

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
                        if tile.tile.state.interactive()
                            && let TileKind::Power = tile.tile.kind
                        {
                            powered = true;
                        }
                        tile.tile.state.interactive() && tile.tile.kind.transmits_power()
                    });
                    if let Some(tile) = self.grid.get_tile_mut(pos)
                        && let TileKind::Light(power) | TileKind::Wire(power) = &mut tile.tile.kind
                    {
                        *power = powered;
                    }
                }
                TileKind::Seed(plant_kind) => {
                    let grow_direction = if self.config.seed_grow_only_up {
                        vec![vec2(0, -1)]
                    } else {
                        Connections::NEIGHBORS.to_vec()
                    };
                    let grow_from = if let PlantKind::TypeC = plant_kind {
                        // Grow from Water
                        grow_direction
                            .iter()
                            .filter_map(|delta| self.grid.get_tile(pos + *delta))
                            .find(|tile| matches!(tile.tile.kind, TileKind::Water(_)))
                            .map(|tile| tile.pos)
                    } else {
                        // Grow from Soil
                        grow_direction
                            .iter()
                            .filter_map(|delta| self.grid.get_tile(pos + *delta))
                            .filter_map(|neighbor| {
                                if let TileKind::Soil(soil_state) = neighbor.tile.kind {
                                    Some((neighbor.pos, soil_state))
                                } else {
                                    None
                                }
                            })
                            .find(|&(_, state)| match plant_kind {
                                PlantKind::TypeA => true,
                                PlantKind::TypeB => state >= SoilState::Watered,
                                PlantKind::TypeC => unreachable!(),
                                PlantKind::TypeD => state >= SoilState::Rich,
                            })
                            .map(|(pos, _)| pos)
                    };
                    if let Some(grow_from) = grow_from
                        && !grow_direction.iter().any(|delta| {
                            if let Some(tile) = self.grid.get_tile(pos - *delta)
                                && let TileKind::Leaf(leaf) = &tile.tile.kind
                            {
                                leaf.kind == plant_kind
                            } else {
                                false
                            }
                        })
                        && let Some(empty) = grow_direction
                            .iter()
                            .map(|delta| pos - *delta)
                            .filter(|&pos| plants::can_grow_into(pos, &self.grid))
                            .choose(&mut rng)
                    {
                        // Grow into a plant
                        if let Some(seed) = self.grid.get_tile_mut(pos) {
                            seed.tile.state.transform();
                        }
                        self.grid.set_tile(
                            empty,
                            Tile::new(TileKind::Leaf(Leaf::new(plant_kind).connected(pos - empty))),
                        );
                        // TODO: gradual usage of water from soil
                        if let Some(grow_from) = self.grid.get_tile_mut(grow_from) {
                            match &mut grow_from.tile.kind {
                                TileKind::Water(_) => grow_from.tile.state.despawn(),
                                TileKind::Soil(soil_state) => {
                                    *soil_state = SoilState::Dry;
                                    grow_from.tile.state.transform();
                                }
                                _ => unreachable!(),
                            }
                        }
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
                TileKind::Water(ref mut lifetime) => {
                    lifetime.change(-delta_time);
                    if lifetime.remaining <= Time::ZERO {
                        // Evaporate
                        tile.tile.state.despawn();
                    }
                }
                TileKind::Bug(ref mut bug) => {
                    if bug.move_timer > Time::ZERO {
                        if let DroneTarget::KillBug(bug_id) = self.drone.target
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
                        let dir = if delta.x.abs() >= delta.y.abs() {
                            vec2(delta.x.signum(), 0)
                        } else {
                            vec2(0, delta.y.signum())
                        };

                        if grid.get_tile(tile.pos + dir).is_none()
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
                                bug.state =
                                    BugState::Pooping(Lifetime::new(self.config.bug_poop_time));
                                continue;
                            }

                            // Look for leaves nearby
                            let leaf_target = self
                                .grid
                                .all_tiles()
                                .filter(|tile| {
                                    if manhattan_distance(pos, tile.pos) <= 7
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
                            let target = leaf_target
                                .or_else(|| {
                                    // Move in available random direction
                                    self.grid
                                        .get_neighbors_all(pos)
                                        .filter(|tile| tile.tile.is_none())
                                        .map(|tile| tile.pos)
                                        .choose(&mut rng)
                                })
                                .unwrap_or(pos);

                            // Go towards target
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
                                // move
                                move_towards(target, &mut self.grid);
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
                                    .filter(|tile| tile.tile.is_none())
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
                            if tile.tile.state.interactive()
                                && let TileKind::Sprinkler(_) = tile.tile.kind
                            {
                                sprinklers.push(tile.pos);
                            }
                            tile.tile.state.interactive() && tile.tile.kind.is_piping()
                        });

                        let empty_tiles: HashSet<vec2<ICoord>> = sprinklers
                            .into_iter()
                            .flat_map(|pos| {
                                self.grid
                                    .get_neighbors_all(pos)
                                    .filter(|tile| tile.tile.is_none())
                                    .map(|tile| tile.pos)
                            })
                            .collect();
                        if let Some(target) = empty_tiles.into_iter().choose(&mut rng) {
                            // Pipe water to a sprinkler
                            if let Some(water) = self.grid.get_tile_mut(water) {
                                water.tile.state.despawn();
                            }
                            self.grid.set_tile(
                                target,
                                Tile::new(TileKind::Water(Lifetime::new(
                                    self.config.water_lifetime,
                                ))),
                            );
                        } else {
                            // Collect water to player inventory
                            self.collect(water);
                        }
                    }
                }
                TileKind::Cutter(_) => {
                    let mut powered = false;
                    get_all_connected(&self.grid, pos, |tile| {
                        if tile.tile.state.interactive()
                            && let TileKind::Power = tile.tile.kind
                        {
                            powered = true;
                        }
                        tile.tile.state.interactive() && tile.tile.kind.transmits_power()
                    });
                    if let Some(tile) = self.grid.get_tile_mut(pos)
                        && let TileKind::Cutter(cutter) = &mut tile.tile.kind
                    {
                        cutter.powered = powered;
                        if powered {
                            cutter.cooldown.change(-delta_time);
                            if cutter.cooldown.remaining <= Time::ZERO {
                                // Cut down a nearby plant
                                cutter.cooldown = Lifetime::new(self.config.cutter_cooldown);
                                let plants: Vec<_> = self
                                    .grid
                                    .all_tiles()
                                    .filter(|tile| {
                                        manhattan_distance(pos, tile.pos)
                                            <= self.config.cutter_radius
                                            && matches!(tile.tile.kind, TileKind::Leaf(_))
                                    })
                                    .map(|tile| tile.pos)
                                    .collect();
                                for plant in plants {
                                    self.cut_plant_tile(plant, true);
                                }
                            }
                        }
                    }
                }
                TileKind::Pipe(_) | TileKind::Sprinkler(_) => {
                    let mut piped = false;
                    get_all_connected(&self.grid, pos, |tile| {
                        if tile.tile.state.interactive()
                            && let TileKind::Drainer = tile.tile.kind
                        {
                            piped = true;
                        }
                        tile.tile.state.interactive() && tile.tile.kind.is_piping()
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

        self.rng_spawn(delta_time);
    }

    /// Cut plant from seed or leaf.
    fn cut_plant_tile(&mut self, target: vec2<ICoord>, earn_money: bool) {
        let Some(tile) = self.grid.get_tile_mut(target) else {
            return;
        };
        let (plant_kind, leaf_connections) = match &tile.tile.kind {
            TileKind::Leaf(leaf) => (leaf.kind, Some(leaf.connections.clone())),
            TileKind::Seed(kind) => (*kind, None),
            _ => return,
        };
        let config = &self.config.plants[&plant_kind];
        tile.tile.state.despawn();
        if leaf_connections.is_some() {
            if earn_money {
                self.money += config.price;
            }
        } else {
            self.inventory_add(TileKind::Seed(plant_kind), 1);
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
                // let mut rooted = false;
                // let group = get_all_connected(&self.grid, tile.pos, |other| {
                //     if other.tile.state.interactive()
                //         && let TileKind::Seed(kind) = other.tile.kind
                //         && kind == plant_kind
                //     {
                //         rooted = true;
                //     }
                //     if target != other.pos
                //         && let TileKind::Leaf(other) = &other.tile.kind
                //         && other.kind == leaf.kind
                //     {
                //         true
                //     } else {
                //         false
                //     }
                // });

                let group = get_whole_plant(&self.grid, tile.pos);
                let rooted = group.iter().any(|&pos| {
                    if let Some(tile) = self.grid.get_tile(pos)
                        && let TileKind::Seed(seed) = tile.tile.kind
                    {
                        seed == plant_kind
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
        let chance = self.config.water_frequency * delta_time;
        if rng.gen_bool(chance.as_f32().into()) {
            // attempt to spawn
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
            for _ in 0..5 {
                if let Some(&anchor) = anchors.choose(&mut rng) {
                    let offset = vec2(rng.gen_range(-2..=2), rng.gen_range(-2..=2));
                    let target = anchor + offset;
                    if self.grid.in_bounds(target) && self.grid.get_tile(target).is_none() {
                        self.grid.set_tile(
                            target,
                            Tile::new(TileKind::Water(Lifetime::new(self.config.water_lifetime))),
                        );
                        break;
                    }
                }
            }
        }

        // Bug
        let chance = self.config.bug_frequency * delta_time;
        if rng.gen_bool(chance.as_f32().into()) {
            // attempt to spawn
            for _ in 0..10 {
                let bounds = self.grid.bounds;
                let pos = vec2(
                    rng.gen_range(bounds.min.x..=bounds.max.x),
                    rng.gen_range(bounds.min.y..=bounds.max.y),
                );
                if self.grid.get_tile(pos).is_none() && !self.grid.is_tile_lit(pos, &self.config) {
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

    fn update_drone(&mut self, delta_time: Time) {
        // Calculate drone's target position
        let target_pos = match self.drone.target {
            DroneTarget::MoveTo(pos)
            | DroneTarget::Interact(pos, _)
            | DroneTarget::PlaceTile(pos, _)
            | DroneTarget::BuyTile(pos, _) => self.grid_visual.tile_bounds(pos).center(),
            DroneTarget::KillBug(bug_id) => {
                let bug = self.grid.tiles.iter().find(|(_, tile)| {
                    if let TileKind::Bug(bug) = &tile.kind
                        && bug.id == bug_id
                    {
                        true
                    } else {
                        false
                    }
                });
                match bug {
                    Some((&pos, _)) => self.grid_visual.tile_bounds(pos).center(),
                    None => {
                        self.drone.target = DroneTarget::MoveTo(
                            self.grid_visual.world_to_grid(self.drone.position),
                        );
                        return;
                    }
                }
            }
        };
        let reach = self.config.drone_reach;
        let offset = (self.drone.position - target_pos).clamp_len(..=reach);
        let target_pos = target_pos + offset;

        let acceleration = self.config.drone_acceleration;
        let deceleration = self.config.drone_deceleration;
        let max_speed = self.config.drone_max_speed;

        let target_dir = target_pos - self.drone.position;
        let target_distance = target_dir.len();

        // Calculate target velocity as the maximum velocity
        // that would not overshoot the target due to deceleration
        let target_velocity = if target_distance.as_f32() < 0.001 {
            vec2::ZERO
        } else {
            let target_speed = (r32(2.0) * deceleration * target_distance)
                .sqrt()
                .min(max_speed);
            target_dir / target_distance * target_speed
        };

        // Update velocity and position
        let relevant_acc = if vec2::dot(self.drone.velocity, target_velocity) < r32(0.0)
            || self.drone.velocity.len_sqr() > target_velocity.len_sqr()
        {
            deceleration * r32(2.0)
        } else {
            acceleration
        };
        self.drone.velocity +=
            (target_velocity - self.drone.velocity).clamp_len(..=relevant_acc * delta_time);
        self.drone.position += self.drone.velocity * delta_time;

        // Action
        if target_distance.as_f32() < 0.001 {
            // target within reach
            self.drone_action(delta_time);
        } else {
            self.drone.action_progress = R32::ZERO;
        }
    }

    fn drone_action(&mut self, delta_time: Time) {
        match self.drone.target.clone() {
            DroneTarget::MoveTo(_) => {}
            DroneTarget::Interact(position, action) => {
                self.drone.action_progress += delta_time / self.config.action_duration[&action];
                if self.drone.action_progress >= R32::ONE {
                    self.drone.action_progress = R32::ZERO;
                    self.drone.target =
                        DroneTarget::MoveTo(self.grid_visual.world_to_grid(self.drone.position));
                    match action {
                        DroneAction::CutPlant => {
                            self.cut_plant_tile(position, true);
                        }
                        DroneAction::Collect => {
                            self.collect(position);
                        }
                        DroneAction::PlaceTile | DroneAction::KillBug => unreachable!(),
                    }
                }
            }
            DroneTarget::KillBug(bug_id) => {
                self.drone.action_progress +=
                    delta_time / self.config.action_duration[&DroneAction::KillBug];
                if self.drone.action_progress >= R32::ONE {
                    self.drone.action_progress = R32::ZERO;
                    self.drone.target =
                        DroneTarget::MoveTo(self.grid_visual.world_to_grid(self.drone.position));
                    let bug = self.grid.tiles.iter().find(|(_, tile)| {
                        if let TileKind::Bug(bug) = &tile.kind
                            && bug.id == bug_id
                        {
                            true
                        } else {
                            false
                        }
                    });
                    if let Some((&pos, _)) = bug
                        && let Some(tile) = self.grid.get_tile_mut(pos)
                    {
                        // Kill bug
                        tile.tile.state.despawn();
                    }
                }
            }
            DroneTarget::PlaceTile(position, tile) => {
                self.drone.action_progress +=
                    delta_time / self.config.action_duration[&DroneAction::PlaceTile];
                if self.drone.action_progress >= R32::ONE {
                    self.drone.action_progress = R32::ZERO;
                    self.drone.target =
                        DroneTarget::MoveTo(self.grid_visual.world_to_grid(self.drone.position));
                    if self.grid.get_tile(position).is_none()
                        && let Some((inv_item_idx, (_, count))) = self
                            .inventory
                            .iter_mut()
                            .enumerate()
                            .find(|(_, (t, _))| *t == tile)
                    {
                        if *count > 1 {
                            *count -= 1;
                        } else {
                            self.inventory.remove(inv_item_idx);
                        }
                        self.grid.set_tile(position, Tile::new(tile.clone()));
                    }
                }
            }
            DroneTarget::BuyTile(position, tile) => {
                self.drone.action_progress +=
                    delta_time / self.config.action_duration[&DroneAction::PlaceTile];
                if self.drone.action_progress >= R32::ONE {
                    self.drone.action_progress = R32::ZERO;
                    self.drone.target =
                        DroneTarget::MoveTo(self.grid_visual.world_to_grid(self.drone.position));
                    let cost = self.config.get_cost(&tile);
                    if self.grid.get_tile(position).is_none() && self.money >= cost {
                        self.grid.set_tile(position, Tile::new(tile.clone()));
                        self.money -= cost;
                    }
                }
            }
        }
    }
}

fn get_whole_plant(grid: &Grid, start: vec2<ICoord>) -> Vec<vec2<ICoord>> {
    let mut connected = vec![start];
    let mut to_check = vec![start];
    while let Some(pos) = to_check.pop() {
        if let Some(tile) = grid.get_tile(pos)
            && tile.tile.state.interactive()
            && let TileKind::Leaf(leaf) = &tile.tile.kind
        {
            let connections: Vec<_> = leaf
                .connections
                .get_connections(tile.pos)
                .map(|other| other.pos)
                .filter(|&other| {
                    !connected.contains(&other)
                        && grid
                            .get_tile(other)
                            .is_some_and(|other| match other.tile.kind {
                                TileKind::Leaf(ref other) => other.kind == leaf.kind,
                                TileKind::Seed(kind) => kind == leaf.kind,
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
