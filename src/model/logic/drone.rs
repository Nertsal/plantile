use super::*;

impl DroneTarget {
    /// Whether the action still makes sense.
    /// For example, trying to collect a water tile that already evaporated is not meaningful.
    pub fn is_relevant(&self, grid: &Grid) -> bool {
        match *self {
            DroneTarget::MoveTo(_) => true,
            DroneTarget::Collect(pos) => grid.get_tile(pos).is_some_and(|tile| {
                tile.tile.kind.is_collectable()
                    && !matches!(tile.tile.state, TileState::Despawning(_))
            }),
            DroneTarget::CutPlant(pos) => grid.get_tile(pos).is_some_and(|tile| {
                matches!(tile.tile.kind, TileKind::Leaf(_) | TileKind::Seed(_))
                    && !matches!(tile.tile.state, TileState::Despawning(_))
            }),
            DroneTarget::PlaceTile(_, _) | DroneTarget::BuyTile(_, _) => true,
            DroneTarget::KillBug(id) => grid.all_tiles().any(|tile| {
                if let TileKind::Bug(bug) = &tile.tile.kind
                    && !matches!(tile.tile.state, TileState::Despawning(_))
                {
                    bug.id == id
                } else {
                    false
                }
            }),
        }
    }

    /// Whether the action can be achieved.
    /// For example collecting tiles at full inventory is not achieveable.
    /// Includes a check for relevancy inside.
    pub fn is_achievable(&self, model: &Model) -> bool {
        if !self.is_relevant(&model.grid) {
            return false;
        }

        match self {
            DroneTarget::MoveTo(_) => true,
            &DroneTarget::Collect(pos) => model.can_collect_at(pos),
            &DroneTarget::CutPlant(pos) => model.grid.get_tile(pos).is_none_or(|tile| {
                matches!(tile.tile.kind, TileKind::Leaf(_))
                    || (matches!(tile.tile.kind, TileKind::Seed(_))
                        && model.can_collect(&tile.tile.kind))
            }),
            DroneTarget::PlaceTile(pos, kind) => {
                model.can_place_tile(kind, false) && model.grid.get_tile(*pos).is_none()
            }
            DroneTarget::BuyTile(pos, kind) => {
                model.can_buy_tile(kind, false) && model.grid.get_tile(*pos).is_none()
            }
            DroneTarget::KillBug(_) => true,
        }
    }
}

impl Model {
    // All active drone actions.
    pub fn all_drone_actions(&self) -> impl Iterator<Item = &DroneTarget> {
        self.drone.target.as_ref().into_iter()
    }

    // All queued actions not yet taken by drones.
    pub fn all_queued_actions(&self) -> impl Iterator<Item = &DroneTarget> {
        self.queued_actions.iter()
    }

    pub fn update_action_queue(&mut self) {
        self.queued_actions
            .retain(|action| action.is_relevant(&self.grid));
    }

    fn get_drone_target_position(&self, target: &Option<DroneTarget>) -> Option<vec2<FCoord>> {
        match target {
            None => Some(self.drone.position),
            Some(target) => match *target {
                DroneTarget::MoveTo(pos)
                | DroneTarget::Collect(pos)
                | DroneTarget::CutPlant(pos)
                | DroneTarget::PlaceTile(pos, _)
                | DroneTarget::BuyTile(pos, _) => Some(self.grid_visual.tile_bounds(pos).center()),
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
                    bug.map(|(pos, _)| self.grid_visual.tile_bounds(*pos).center())
                }
            },
        }
    }

    pub fn update_drone_position(&mut self, delta_time: Time) {
        // Calculate drone's target position
        let Some(target_pos) = self.get_drone_target_position(&self.drone.target) else {
            self.drone.target = None;
            return;
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
        let delta_velocity =
            (target_velocity - self.drone.velocity).clamp_len(..=relevant_acc * delta_time);
        self.drone.velocity += delta_velocity;
        self.drone.position +=
            (self.drone.velocity + delta_velocity * delta_time / r32(2.0)) * delta_time;
    }

    pub fn update_drone(&mut self, delta_time: Time) {
        // Update drone target
        // Look for jobs
        if self
            .drone
            .target
            .as_ref()
            .is_none_or(|target| !target.is_relevant(&self.grid))
        {
            self.drone.target = None;
            if let Some(i) = self
                .queued_actions
                .iter()
                .position(|action| action.is_achievable(self))
            {
                self.drone.target = self.queued_actions.remove(i);
            }
        }

        let Some(target_pos) = self.get_drone_target_position(&self.drone.target) else {
            self.drone.target = None;
            return;
        };
        let target_distance = (target_pos - self.drone.position).len();

        // Action
        if target_distance - r32(0.01) <= self.config.drone_reach {
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

        // Timer progress
        let mut action_finish = false;
        if let Some(action) = target.action() {
            self.drone.action_progress += delta_time / self.config.action_duration[&action];
            if self.drone.action_progress >= R32::ONE {
                self.drone.action_progress = R32::ZERO;
                action_finish = true
            }
        }

        match target {
            DroneTarget::MoveTo(_) => {
                // Stop once reached
                self.drone.target = None;
            }
            DroneTarget::Collect(position) => {
                if action_finish {
                    self.drone.target = None;
                    self.collect(position);
                }
            }
            DroneTarget::CutPlant(position) => {
                if action_finish {
                    self.drone.target = None;
                    self.cut_plant_tile(position, true);
                }
            }
            DroneTarget::KillBug(bug_id) => {
                if action_finish {
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
                if action_finish {
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
                if action_finish {
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
