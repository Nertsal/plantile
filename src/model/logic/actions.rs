use super::*;

impl Model {
    pub fn interact_with(&mut self, target: vec2<ICoord>, allow_move: bool) -> Option<DroneTarget> {
        if self.active_action_at(target).is_some() {
            // Cannot interact with ghosts
            return None;
        }

        let interaction = self.tile_interaction(target);
        if let DroneTarget::MoveTo(_) = interaction {
            // Tell the closest unoccupied drone to move here
            if allow_move
                && self
                    .drone
                    .target
                    .as_ref()
                    .is_none_or(|target| matches!(target, DroneTarget::MoveTo(_)))
            {
                self.drone.target = Some(interaction.clone());
                self.context
                    .sfx
                    .play(&self.context.assets.sounds.drone_confirm);
                return Some(interaction);
            }
        } else {
            log::debug!("interact with {}: {:?}", target, interaction);
            if interaction.is_relevant(&self.grid) {
                self.queued_actions.push_back(interaction.clone());
                self.context
                    .sfx
                    .play(&self.context.assets.sounds.drone_confirm);
                return Some(interaction);
            }
        }

        None
    }

    pub fn tile_interaction(&self, target: vec2<ICoord>) -> DroneTarget {
        let Some(tile) = self.grid.get_tile(target) else {
            // Tell the drone to just fly to this tile
            return DroneTarget::MoveTo(target);
        };

        match &tile.tile.kind {
            TileKind::Leaf(_) | TileKind::Seed(_) => DroneTarget::CutPlant(target),
            TileKind::Bug(bug) => DroneTarget::KillBug(bug.id),
            _ if tile.tile.kind.is_collectable() => {
                if !self.can_collect(&tile.tile.kind) {
                    // Inventory already maxed
                    // self.context
                    //     .sfx
                    //     .play(&self.context.assets.sounds.drone_deny);
                    return DroneTarget::MoveTo(target);
                }
                DroneTarget::Collect(target)
            }
            _ => DroneTarget::MoveTo(target),
        }
    }

    fn can_build_at(&self, target: vec2<ICoord>) -> bool {
        self.active_action_at(target).is_none()
            && self
                .grid
                .get_tile(target)
                .is_none_or(|tile| matches!(tile.tile.state, TileState::Despawning { .. }))
    }

    /// If `count_queued` is true, also accounts for the queued actions not yet taken by drones.
    pub fn can_place_tile(&self, tile: &TileKind, count_queued: bool) -> bool {
        let mut queued = 0;
        let mut process = |target: &DroneTarget| {
            if let DroneTarget::PlaceTile(_, kind) = target
                && kind == tile
            {
                queued += 1;
            }
        };

        for target in self.all_drone_actions() {
            process(target);
        }
        if count_queued {
            for target in self.all_queued_actions() {
                process(target);
            }
        }

        let available = self.inventory.get(tile).copied().unwrap_or(0);
        available > queued
    }

    pub fn place_tile(&mut self, target: vec2<ICoord>, tile: TileKind) -> bool {
        if !self.can_build_at(target) || !self.inventory.iter().any(|(t, _)| *t == tile) {
            return false;
        }

        log::debug!("place tile at {}: {:?}", target, tile);

        self.queued_actions
            .push_back(DroneTarget::PlaceTile(target, tile));
        self.context
            .sfx
            .play(&self.context.assets.sounds.drone_confirm);

        true
    }

    /// If `count_queued` is true, also accounts for the queued actions not yet taken by drones.
    pub fn can_buy_tile(&self, tile: &TileKind, count_queued: bool) -> bool {
        let mut queued_cost = 0;
        let mut process = |target: &DroneTarget| {
            if let DroneTarget::BuyTile(_, kind) = target {
                queued_cost += self.config.get_cost(kind);
            }
        };

        for target in self.all_drone_actions() {
            process(target);
        }
        if count_queued {
            for target in self.all_queued_actions() {
                process(target);
            }
        }

        self.money >= queued_cost + self.config.get_cost(tile)
    }

    pub fn buy_tile(&mut self, target: vec2<ICoord>, tile: TileKind) -> bool {
        if !self.can_build_at(target) || self.money < self.config.get_cost(&tile) {
            return false;
        }

        log::debug!("buy tile at {}: {:?}", target, tile);

        self.queued_actions
            .push_back(DroneTarget::BuyTile(target, tile));
        self.context
            .sfx
            .play(&self.context.assets.sounds.drone_confirm);

        true
    }

    pub fn can_collect(&self, kind: &TileKind) -> bool {
        let kind = tile_kind_normalization(kind.clone());
        self.inventory.len() < INVENTORY_MAX_SIZE || self.inventory.contains_key(&kind)
    }

    pub fn can_collect_at(&self, target: vec2<ICoord>) -> bool {
        let Some(tile) = self.grid.get_tile(target) else {
            return false;
        };
        self.can_collect(&tile.tile.kind)
    }

    pub fn collect(&mut self, target: vec2<ICoord>, despawn_into: Option<vec2<FCoord>>) {
        let Some(tile) = self.grid.get_tile(target) else {
            return;
        };
        log::debug!("collect {}: {:?}", target, tile.tile);

        if !self.can_collect(&tile.tile.kind) {
            return;
        }

        let Some(tile) = self.grid.get_tile_mut(target) else {
            return;
        };

        if tile.tile.kind.is_collectable() {
            if let Some(pos) = despawn_into {
                tile.tile.state.despawn_into(pos);
            } else {
                tile.tile.state.despawn();
            }
            let kind = tile.tile.kind.clone();
            self.inventory_add(kind, 1);
            self.context.sfx.play(&self.context.assets.sounds.collect);
        }
    }

    pub fn inventory_add(&mut self, kind: TileKind, count: usize) {
        let kind = tile_kind_normalization(kind);
        *self.inventory.entry(kind).or_insert(0) += count;
    }
}

fn tile_kind_normalization(mut kind: TileKind) -> TileKind {
    match &mut kind {
        TileKind::Water(lifetime) | TileKind::Poop(lifetime) => {
            lifetime.remaining = lifetime.max;
        }
        TileKind::Light(powered) | TileKind::Wire(powered) => *powered = false,
        TileKind::Pipe(connected) | TileKind::Sprinkler(connected) => *connected = false,
        TileKind::Cutter(cutter) => *cutter = Cutter::default(),
        TileKind::Seed(seed) => *seed = Seed::new(seed.kind),
        _ => {}
    }
    kind
}
