use super::*;

impl Model {
    // All queued and active drone actions.
    pub fn all_actions(&self) -> impl Iterator<Item = &DroneTarget> {
        itertools::chain![&self.drone.target, &self.queued_actions]
    }

    pub fn update_drone(&mut self, delta_time: Time) {
        // Update drone target
        if self.drone.target.is_none() {
            // Look for jobs
            self.drone.target = self.queued_actions.pop_front();
        }

        // Calculate drone's target position
        let target_pos = match &self.drone.target {
            None => self.drone.position,
            Some(target) => match *target {
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
                            self.drone.target = None;
                            return;
                        }
                    }
                }
            },
        };

        // Go towards target position
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

    pub fn drone_action(&mut self, delta_time: Time) {
        let Some(target) = self.drone.target.clone() else {
            return;
        };
        match target {
            DroneTarget::MoveTo(_) => {
                // Stop once reached
                self.drone.target = None;
            }
            DroneTarget::Interact(position, action) => {
                self.drone.action_progress += delta_time / self.config.action_duration[&action];
                if self.drone.action_progress >= R32::ONE {
                    self.drone.action_progress = R32::ZERO;
                    self.drone.target = None;
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
                    self.drone.target = None;
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
                    self.drone.target = None;
                    if self.grid.get_tile(position).is_none()
                        && let Some(count) = self.inventory.get_mut(&tile)
                    {
                        if *count > 1 {
                            *count -= 1;
                        } else {
                            self.inventory.remove(&tile);
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
                    self.drone.target = None;
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
