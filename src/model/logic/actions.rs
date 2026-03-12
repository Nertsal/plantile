use super::*;

impl Model {
    pub fn interact_with(&mut self, target: vec2<ICoord>) {
        log::debug!("interact with {}", target);
        self.queued_actions.push_back(self.tile_interaction(target));
        self.context
            .sfx
            .play(&self.context.assets.sounds.drone_confirm);
    }

    pub fn tile_interaction(&self, target: vec2<ICoord>) -> DroneTarget {
        let Some(tile) = self.grid.get_tile(target) else {
            // Tell the drone to just fly to this tile
            return DroneTarget::MoveTo(target);
        };

        match &tile.tile.kind {
            TileKind::Leaf(_) | TileKind::Seed(_) => {
                DroneTarget::Interact(target, DroneAction::CutPlant)
            }
            TileKind::Bug(bug) => DroneTarget::KillBug(bug.id),
            _ if tile.tile.kind.is_collectable() => {
                if self.inventory.len() >= INVENTORY_MAX_SIZE {
                    // Inventory already maxed
                    // self.context
                    //     .sfx
                    //     .play(&self.context.assets.sounds.drone_deny);
                    return DroneTarget::MoveTo(target);
                }
                DroneTarget::Interact(target, DroneAction::Collect)
            }
            _ => DroneTarget::MoveTo(target),
        }
    }

    pub fn can_place_tile(&self, tile: &TileKind) -> bool {
        let queued = self
            .all_actions()
            .filter(|target| {
                if let DroneTarget::PlaceTile(_, kind) = target
                    && kind == tile
                {
                    true
                } else {
                    false
                }
            })
            .count();
        let available = self.inventory.get(tile).copied().unwrap_or(0);
        available > queued
    }

    pub fn place_tile(&mut self, target: vec2<ICoord>, tile: TileKind) -> bool {
        log::debug!("place tile at {}: {:?}", target, tile);
        if self.grid.get_tile(target).is_some() || !self.inventory.iter().any(|(t, _)| *t == tile) {
            return false;
        }

        self.queued_actions
            .push_back(DroneTarget::PlaceTile(target, tile));
        self.context
            .sfx
            .play(&self.context.assets.sounds.drone_confirm);

        true
    }

    pub fn can_buy_tile(&self, tile: &TileKind) -> bool {
        let queued_cost: Money = self
            .all_actions()
            .filter_map(|target| {
                if let DroneTarget::BuyTile(_, kind) = target {
                    Some(self.config.get_cost(kind))
                } else {
                    None
                }
            })
            .sum();
        self.money > queued_cost + self.config.get_cost(tile)
    }

    pub fn buy_tile(&mut self, target: vec2<ICoord>, tile: TileKind) -> bool {
        log::debug!("buy tile at {}: {:?}", target, tile);
        if self.grid.get_tile(target).is_some() {
            return false;
        }

        let cost = self.config.get_cost(&tile);
        if self.money < cost {
            return false;
        }

        self.queued_actions
            .push_back(DroneTarget::BuyTile(target, tile));
        self.context
            .sfx
            .play(&self.context.assets.sounds.drone_confirm);

        true
    }

    pub fn collect(&mut self, target: vec2<ICoord>) {
        if self.inventory.len() >= INVENTORY_MAX_SIZE {
            // Inventory already maxed
            return;
        }

        let Some(tile) = self.grid.get_tile_mut(target) else {
            return;
        };
        log::debug!("collect {}: {:?}", target, tile.tile);

        if tile.tile.kind.is_collectable() {
            tile.tile.state.despawn();
            let mut kind = tile.tile.kind.clone();
            match &mut kind {
                TileKind::Water(lifetime) | TileKind::Poop(lifetime) => {
                    *lifetime = Lifetime::new(self.config.water_lifetime);
                }
                TileKind::Light(powered)
                | TileKind::Wire(powered)
                | TileKind::Sprinkler(powered) => *powered = false,
                TileKind::Cutter(cutter) => *cutter = Cutter::default(),
                TileKind::Seed(seed) => seed.growth_energy.clear(),
                _ => {}
            }
            self.inventory_add(kind, 1);
            self.context.sfx.play(&self.context.assets.sounds.rock);
        }
    }

    pub fn inventory_add(&mut self, tile: TileKind, count: usize) {
        *self.inventory.entry(tile).or_insert(0) += count;
    }
}
