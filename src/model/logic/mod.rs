mod actions;
mod plants;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: Time) {
        self.update_drone(delta_time);

        // Update tiles
        let update_order: Vec<vec2<ICoord>> = self.grid.all_positions().collect();
        for pos in update_order {
            let Some(tile) = self.grid.get_tile(pos) else {
                continue;
            };
            match &tile.tile {
                Tile::Leaf(_) => self.update_plant(tile.pos, delta_time),
                Tile::Light => {}
                Tile::Seed(plant_kind) => {
                    let soil = self
                        .grid
                        .get_neighbors(tile.pos)
                        .find(|neighbor| matches!(neighbor.tile, Tile::Soil(_)))
                        .map(|neighbor| neighbor.pos);
                    if let Some(soil) = soil {
                        // Grow into a plant
                        self.grid
                            .set_tile(tile.pos, Tile::Leaf(Leaf::new(*plant_kind).root()));
                        self.grid.set_tile(soil, Tile::Soil(SoilState::Dry));
                    }
                }
                Tile::Soil(_state) => {}
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
