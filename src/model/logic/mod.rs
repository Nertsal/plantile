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
            .sorted_by_key(|tile| tile.tile.update_order())
            .map(|tile| tile.pos)
            .collect();
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
                    if let PlantKind::TypeC = plant_kind {
                        // Grow from water
                        let water = self
                            .grid
                            .get_neighbors(pos)
                            .find(|tile| matches!(tile.tile, Tile::Water(_)))
                            .map(|tile| tile.pos);
                        if let Some(water) = water {
                            // Grow into a plant
                            self.grid
                                .set_tile(pos, Tile::Leaf(Leaf::new(plant_kind).root()));
                            self.grid.remove_tile(water);
                        }
                        continue;
                    }

                    // Grow from soil
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
                            PlantKind::TypeC => unreachable!(),
                            PlantKind::TypeD => state >= SoilState::Rich,
                        });
                    if let Some((soil_pos, _soil_state)) = soil {
                        // Grow into a plant
                        self.grid
                            .set_tile(pos, Tile::Leaf(Leaf::new(plant_kind).root()));
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
                    SoilState::Watered => {
                        let poop = self
                            .grid
                            .get_neighbors(pos)
                            .find(|tile| matches!(tile.tile, Tile::Poop(_)));
                        if let Some(poop) = poop {
                            self.grid.remove_tile(poop.pos);
                            let soil = self.grid.get_tile_mut(pos).unwrap();
                            if let Tile::Soil(state) = soil.tile {
                                *state = SoilState::Rich;
                            }
                        }
                    }
                    SoilState::Rich => {}
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

                        if grid.get_tile(tile.pos + dir).is_none()
                            && let Some(mut tile) = grid.remove_tile(pos)
                            && let Tile::Bug(bug) = &mut tile.tile
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
                                    self.context.sfx.play(&self.context.assets.sounds.bug_poop);
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
                Tile::Drainer => {
                    let water = self
                        .grid
                        .all_tiles()
                        .find(|tile| {
                            // Collect water within range not adjacent to a sprinkler
                            matches!(tile.tile, Tile::Water(_))
                                && manhattan_distance(pos, tile.pos) <= self.config.drainer_radius
                                && !self
                                    .grid
                                    .get_neighbors(tile.pos)
                                    .any(|tile| matches!(tile.tile, Tile::Sprinkler(_)))
                        })
                        .map(|tile| tile.pos);
                    if let Some(water) = water {
                        // Look for a sprinkler
                        let mut sprinklers = Vec::new();
                        get_all_connected(&self.grid, pos, |tile| {
                            if let Tile::Sprinkler(_) = tile.tile {
                                sprinklers.push(tile.pos);
                            }
                            tile.tile.is_piping()
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
                            self.grid.remove_tile(water);
                            self.grid
                                .set_tile(target, Tile::Water(self.config.water_lifetime));
                        } else {
                            // Collect water to player inventory
                            self.collect(water);
                        }
                    }
                }
                Tile::Cutter(_) => {
                    let mut powered = false;
                    get_all_connected(&self.grid, pos, |tile| {
                        if let Tile::Power = tile.tile {
                            powered = true;
                        }
                        tile.tile.transmits_power()
                    });
                    if let Some(tile) = self.grid.get_tile_mut(pos)
                        && let Tile::Cutter(cutter) = tile.tile
                    {
                        cutter.powered = powered;
                        if powered {
                            cutter.cooldown -= delta_time;
                            if cutter.cooldown <= Time::ZERO {
                                // Cut down a nearby plant
                                cutter.cooldown = self.config.cutter_cooldown;
                                let plant = self
                                    .grid
                                    .all_tiles()
                                    .find(|tile| {
                                        manhattan_distance(pos, tile.pos)
                                            <= self.config.cutter_radius
                                            && matches!(tile.tile, Tile::Leaf(_))
                                    })
                                    .map(|tile| tile.pos);
                                if let Some(plant) = plant {
                                    self.cut_plant_tile(plant, true);
                                }
                            }
                        }
                    }
                }
                Tile::Pipe(_) | Tile::Sprinkler(_) => {
                    let mut piped = false;
                    get_all_connected(&self.grid, pos, |tile| {
                        if let Tile::Drainer = tile.tile {
                            piped = true;
                        }
                        tile.tile.is_piping()
                    });
                    if let Some(tile) = self.grid.get_tile_mut(pos)
                        && let Tile::Pipe(connected) | Tile::Sprinkler(connected) = tile.tile
                    {
                        *connected = piped;
                    }
                }
                Tile::Rock => {}
            }
        }

        self.rng_spawn(delta_time);
    }

    fn cut_plant_tile(&mut self, target: vec2<ICoord>, earn_money: bool) {
        let Some(tile) = self.grid.get_tile(target) else {
            return;
        };
        let Tile::Leaf(leaf) = tile.tile else { return };
        let config = &self.config.plants[&leaf.kind];
        if earn_money {
            self.money += config.price;
        }
        self.grid.remove_tile(target);

        let mut lost_plants = Vec::new();
        for tile in self.grid.get_neighbors(target) {
            if lost_plants.contains(&tile.pos) {
                continue;
            }
            if let Tile::Leaf(leaf) = tile.tile
                && !leaf.root
            {
                // Check connectivity to root
                let mut rooted = false;
                let group = get_all_connected(&self.grid, tile.pos, |other| {
                    if let Tile::Leaf(other) = other.tile
                        && other.kind == leaf.kind
                    {
                        if other.root {
                            rooted = true;
                        }
                        true
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
            if let Some(tile) = self.grid.remove_tile(tile)
                && earn_money
                && let Tile::Leaf(leaf) = tile.tile
            {
                let config = &self.config.plants[&leaf.kind];
                self.money += config.price;
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
                    self.grid.set_tile(pos, Tile::Rock);
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
                        if let Tile::Leaf(leaf) = tile.tile {
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
                    if self.grid.get_tile(target).is_none() {
                        self.grid
                            .set_tile(target, Tile::Water(self.config.water_lifetime));
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
                            self.cut_plant(position);
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
                        self.grid.set_tile(position, tile.clone());
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

// pub fn aabb_contains(aabb: Aabb2<ICoord>, pos: vec2<ICoord>) -> bool {
//     aabb.min.x <= pos.x && aabb.min.y <= pos.y && aabb.max.x >= pos.x && aabb.max.y >= pos.y
// }

pub fn manhattan_distance(a: vec2<ICoord>, b: vec2<ICoord>) -> ICoord {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
