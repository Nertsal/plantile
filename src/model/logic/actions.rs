use super::*;

impl Model {
    pub fn interact_with(&mut self, target: vec2<ICoord>) {
        log::debug!("interact with {}", target);
        let Some(tile) = self.grid.get_tile(target) else {
            // Tell the drone to just fly to this tile
            self.drone.target = DroneTarget::MoveTo(target);
            return;
        };

        self.drone.target = match &tile.tile.kind {
            TileKind::Leaf(_) => DroneTarget::Interact(target, DroneAction::CutPlant),
            TileKind::Bug(bug) => DroneTarget::KillBug(bug.id),
            _ if tile.tile.kind.is_collectable() => {
                if self.inventory.len() >= INVENTORY_MAX_SIZE {
                    // Inventory already maxed
                    // self.context
                    //     .sfx
                    //     .play(&self.context.assets.sounds.drone_deny);
                    self.drone.target = DroneTarget::MoveTo(target);
                    return;
                }
                DroneTarget::Interact(target, DroneAction::Collect)
            }
            _ => DroneTarget::MoveTo(target),
        };
        self.context
            .sfx
            .play(&self.context.assets.sounds.drone_confirm);
    }

    pub fn place_tile(&mut self, target: vec2<ICoord>, tile: TileKind) -> bool {
        log::debug!("place tile at {}: {:?}", target, tile);
        if self.grid.get_tile(target).is_some() || !self.inventory.iter().any(|(t, _)| *t == tile) {
            return false;
        }

        self.drone.target = DroneTarget::PlaceTile(target, tile);
        self.context
            .sfx
            .play(&self.context.assets.sounds.drone_confirm);

        true
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

        self.drone.target = DroneTarget::BuyTile(target, tile);
        self.context
            .sfx
            .play(&self.context.assets.sounds.drone_confirm);

        true
    }

    pub fn cut_plant(&mut self, target: vec2<ICoord>) -> bool {
        log::debug!("cut plant at {}", target);
        let Some(tile) = self.grid.get_tile(target) else {
            return false;
        };
        let TileKind::Leaf(leaf) = &tile.tile.kind else {
            return false;
        };

        let plant_positions = get_all_connected(&self.grid, target, |tile| {
            if let TileKind::Leaf(other) = tile.tile
                && leaf.kind == other.kind
            {
                true
            } else {
                false
            }
        });

        // Earn money
        let size = plant_positions.len();
        self.money += size as Money * self.config.plants[&leaf.kind].price;

        // Remove stem and leaves
        for pos in plant_positions {
            if let Some(mut tile) = self.grid.remove_tile(pos)
                && let TileKind::Leaf(leaf) = &mut tile.tile.kind
                && leaf.root
            {
                // Replace root with a new seed
                // TODO: actually this is outdated behavior
                self.grid
                    .set_tile(pos, Tile::new(TileKind::Seed(leaf.kind)));
            }
        }

        self.context.sfx.play(&self.context.assets.sounds.rock);

        true
    }

    pub fn collect(&mut self, target: vec2<ICoord>) {
        if self.inventory.len() >= INVENTORY_MAX_SIZE {
            // Inventory already maxed
            return;
        }

        let Some(tile) = self.grid.get_tile(target) else {
            return;
        };
        log::debug!("collect {}: {:?}", target, tile.tile);

        if tile.tile.kind.is_collectable() {
            let mut tile = self.grid.remove_tile(target).unwrap();
            match &mut tile.tile.kind {
                TileKind::Water(lifetime) | TileKind::Poop(lifetime) => {
                    *lifetime = Lifetime::new(self.config.water_lifetime);
                }
                TileKind::Light(powered) | TileKind::Wire(powered) => *powered = false,
                _ => {}
            }
            self.inventory_add(tile.tile.kind, 1);
            self.context.sfx.play(&self.context.assets.sounds.rock);
        }
    }

    pub fn inventory_add(&mut self, tile: TileKind, count: usize) {
        match self.inventory.iter_mut().find(|(t, _)| *t == tile) {
            Some((_, available)) => *available += count,
            None => self.inventory.push((tile, count)),
        }
    }
}
