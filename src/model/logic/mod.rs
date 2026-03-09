mod actions;
mod plants;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: Time) {
        let mut rng = thread_rng();

        self.update_drone(delta_time);

        // Update tiles
        let update_order: Vec<vec2<ICoord>> = self.grid.all_positions().collect();
        for pos in update_order {
            let Some(tile) = self.grid.get_tile_mut(pos) else {
                continue;
            };
            match *tile.tile {
                Tile::Leaf(_) => self.update_plant(pos, delta_time),
                Tile::Power => {}
                Tile::Light(_) | Tile::Wire(_) => {
                    let mut powered = false;
                    get_all_connected(&self.grid, pos, |tile| {
                        if let Tile::Power = tile.tile {
                            powered = true;
                        }
                        tile.tile.transmits_power()
                    });
                    if let Some(tile) = self.grid.get_tile_mut(pos)
                        && let Tile::Light(power) | Tile::Wire(power) = tile.tile
                    {
                        *power = powered;
                    }
                }
                Tile::Seed(plant_kind) => {
                    let soil = self
                        .grid
                        .get_neighbors(pos)
                        .filter_map(|neighbor| {
                            if let Tile::Soil(soil_state) = neighbor.tile {
                                Some((neighbor.pos, *soil_state))
                            } else {
                                None
                            }
                        })
                        .find(|&(_, state)| match plant_kind {
                            PlantKind::TypeA => true,
                            PlantKind::TypeB => state >= SoilState::Watered,
                        });
                    if let Some((soil_pos, _soil_state)) = soil {
                        // Grow into a plant
                        self.grid.set_tile(
                            pos,
                            Tile::Leaf(Leaf::new(plant_kind, self.config.plant_growth_time).root()),
                        );
                        self.grid.set_tile(soil_pos, Tile::Soil(SoilState::Dry));
                    }
                }
                Tile::Soil(state) => match state {
                    SoilState::Dry => {
                        let water = self
                            .grid
                            .get_neighbors(pos)
                            .find(|tile| matches!(tile.tile, Tile::Water(_)));
                        if let Some(water) = water {
                            self.grid.remove_tile(water.pos);
                            let soil = self.grid.get_tile_mut(pos).unwrap();
                            if let Tile::Soil(state) = soil.tile {
                                *state = SoilState::Watered;
                            }
                        }
                    }
                    SoilState::Watered => {}
                },
                Tile::Water(ref mut lifetime) => {
                    *lifetime -= delta_time;
                    if *lifetime <= Time::ZERO {
                        // Evaporate
                        self.grid.remove_tile(pos);
                    }
                }
                Tile::Bug(ref mut bug) => {
                    if bug.move_timer > Time::ZERO {
                        bug.move_timer -= delta_time;
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

                        if let Some(mut tile) = grid.remove_tile(pos)
                            && let Tile::Bug(bug) = &mut tile.tile
                            && grid.get_tile(tile.pos + dir).is_none()
                        {
                            bug.move_timer = self.config.bug_move_time;
                            grid.set_tile(tile.pos + dir, tile.tile);
                        }
                    };

                    match &mut bug.state {
                        BugState::Hungry { hunger, .. } => {
                            if *hunger == 0 {
                                bug.state = BugState::Pooping(self.config.bug_poop_time);
                                continue;
                            }

                            // Look for leaves nearby
                            let leaf_target = self
                                .grid
                                .all_tiles()
                                .filter(|tile| {
                                    if manhattan_distance(pos, tile.pos) <= 7
                                        && let Tile::Leaf(leaf) = tile.tile
                                        && !leaf.root
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
                                && let Tile::Leaf(_) = tile.tile
                            {
                                // eat
                                if let Some(bug) = self.grid.get_tile_mut(pos)
                                    && let Tile::Bug(bug) = bug.tile
                                    && let BugState::Hungry {
                                        eating_timer,
                                        hunger,
                                    } = &mut bug.state
                                {
                                    *eating_timer -= delta_time;
                                    if *eating_timer <= Time::ZERO {
                                        *eating_timer = self.config.bug_eat_time;
                                        *hunger -= 1;
                                        self.grid.remove_tile(target);
                                    }
                                }
                            } else {
                                // move
                                move_towards(target, &mut self.grid);
                            }
                        }
                        BugState::Pooping(timer) => {
                            *timer -= delta_time;
                            if *timer <= Time::ZERO {
                                let target = self
                                    .grid
                                    .get_neighbors_all(pos)
                                    .find(|tile| tile.tile.is_none())
                                    .map(|tile| tile.pos);
                                if let Some(target) = target {
                                    self.grid
                                        .set_tile(target, Tile::Poop(self.config.poop_lifetime));
                                    if let Some(bug) = self.grid.get_tile_mut(pos)
                                        && let Tile::Bug(bug) = bug.tile
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
                                    eating_timer: self.config.bug_eat_time,
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
                Tile::Poop(ref mut lifetime) => {
                    *lifetime -= delta_time;
                    if *lifetime <= Time::ZERO {
                        self.grid.remove_tile(pos);
                    }
                }
            }
        }

        self.rng_spawn(delta_time);
    }

    fn rng_spawn(&mut self, delta_time: Time) {
        let mut rng = thread_rng();

        // Water
        let chance = self.config.water_frequency * delta_time;
        if rng.gen_bool(chance.as_f32().into()) {
            // attempt to spawn
            let anchors = self.grid.all_positions().filter(|pos| {
                self.grid.get_tile(*pos).is_some_and(|tile| {
                    if let Tile::Leaf(leaf) = tile.tile {
                        leaf.growth_timer.is_some()
                    } else {
                        false
                    }
                })
            });
            if let Some(anchor) = anchors.choose(&mut rng) {
                let offset = vec2(rng.gen_range(-2..=2), rng.gen_range(-2..=2));
                let target = anchor + offset;
                if self.grid.get_tile(target).is_none() {
                    self.grid
                        .set_tile(target, Tile::Water(self.config.water_lifetime));
                }
            }
        }

        // Bug
        let chance = self.config.bug_frequency * delta_time;
        if rng.gen_bool(chance.as_f32().into()) {
            // attempt to spawn
            let pos = vec2(rng.gen_range(-10..=10), rng.gen_range(0..10));
            if self.grid.get_tile(pos).is_none() {
                self.grid.set_tile(
                    pos,
                    Tile::Bug(Bug {
                        id: self.next_id,
                        state: BugState::Hungry {
                            hunger: self.config.bug_hunger,
                            eating_timer: self.config.bug_eat_time,
                        },
                        move_timer: self.config.bug_move_time,
                    }),
                );
                self.next_id += 1;
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
                    if let Tile::Bug(bug) = tile
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
        let reach = r32(Drone::REACH);
        let offset = (self.drone.position - target_pos).clamp_len(..=reach);
        let target_pos = target_pos + offset;

        let acceleration = r32(Drone::ACCELERATION);
        let deceleration = r32(Drone::DECELERATION);
        let max_speed = r32(Drone::MAX_SPEED);

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
                self.drone.action_progress += delta_time; // / self.config.action_duration[action];
                if self.drone.action_progress >= R32::ONE {
                    self.drone.action_progress = R32::ZERO;
                    self.drone.target =
                        DroneTarget::MoveTo(self.grid_visual.world_to_grid(self.drone.position));
                    match action {
                        DroneAction::CutPlant => {
                            self.cut_plant(position);
                        }
                        DroneAction::Collect => {
                            self.collect(position);
                        }
                    }
                }
            }
            DroneTarget::KillBug(bug_id) => {
                self.drone.action_progress += delta_time;
                if self.drone.action_progress >= R32::ONE {
                    self.drone.action_progress = R32::ZERO;
                    self.drone.target =
                        DroneTarget::MoveTo(self.grid_visual.world_to_grid(self.drone.position));
                    let bug = self.grid.tiles.iter().find(|(_, tile)| {
                        if let Tile::Bug(bug) = tile
                            && bug.id == bug_id
                        {
                            true
                        } else {
                            false
                        }
                    });
                    if let Some((&pos, _)) = bug {
                        // Kill bug
                        self.grid.remove_tile(pos);
                    }
                }
            }
            DroneTarget::PlaceTile(position, tile) => {
                self.drone.action_progress += delta_time; // / self.config.action_duration[action];
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
                        self.grid.set_tile(position, tile.clone());
                    }
                }
            }
            DroneTarget::BuyTile(position, tile) => {
                self.drone.action_progress += delta_time; // / self.config.action_duration[action];
                if self.drone.action_progress >= R32::ONE {
                    self.drone.action_progress = R32::ZERO;
                    self.drone.target =
                        DroneTarget::MoveTo(self.grid_visual.world_to_grid(self.drone.position));
                    let cost = self.config.get_cost(&tile);
                    if self.grid.get_tile(position).is_none() && self.money >= cost {
                        self.grid.set_tile(position, tile.clone());
                        self.money -= cost;
                    }
                }
            }
        }
    }
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

pub fn aabb_contains(aabb: Aabb2<ICoord>, pos: vec2<ICoord>) -> bool {
    aabb.min.x <= pos.x && aabb.min.y <= pos.y && aabb.max.x >= pos.x && aabb.max.y >= pos.y
}

pub fn manhattan_distance(a: vec2<ICoord>, b: vec2<ICoord>) -> ICoord {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
