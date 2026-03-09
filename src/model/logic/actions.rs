use super::*;

impl Model {
    pub fn interact_with(&mut self, target: vec2<ICoord>) {
        log::debug!("interact with {}", target);
        let Some(tile) = self.grid.get_tile(target) else {
            // Tell the drone to just fly to this tile
            self.drone.target = DroneTarget::MoveTo(target);
            return;
        };

        self.drone.target = match &tile.tile {
            Tile::Leaf(_) => DroneTarget::Interact(target, DroneAction::CutPlant),
            Tile::Bug(bug) => DroneTarget::KillBug(bug.id),
            _ if tile.tile.is_collectable() => DroneTarget::Interact(target, DroneAction::Collect),
            _ => DroneTarget::MoveTo(target),
        };
    }

    pub fn place_tile(&mut self, target: vec2<ICoord>, tile: Tile) -> bool {
        log::debug!("place tile at {}: {:?}", target, tile);
        if self.grid.get_tile(target).is_some() || !self.inventory.iter().any(|(t, _)| *t == tile) {
            return false;
        }

        self.drone.target = DroneTarget::PlaceTile(target, tile);

        true
    }

    pub fn buy_tile(&mut self, target: vec2<ICoord>, tile: Tile) -> bool {
        log::debug!("buy tile at {}: {:?}", target, tile);
        if self.grid.get_tile(target).is_some() {
            return false;
        }

        let cost = self.config.get_cost(&tile);
        if self.money < cost {
            return false;
        }

        self.drone.target = DroneTarget::BuyTile(target, tile);

        true
    }

    pub fn cut_plant(&mut self, target: vec2<ICoord>) -> bool {
        log::debug!("cut plant at {}", target);
        let Some(tile) = self.grid.get_tile(target) else {
            return false;
        };
        let Tile::Leaf(_) = tile.tile else {
            return false;
        };

        let plant_positions = get_all_connected(&self.grid, target, |tile| {
            matches!(tile.tile, Tile::Leaf(_))
        });

        // Earn money
        let size = plant_positions.len();
        self.money += size as Money;

        // Remove stem and leaves
        for pos in plant_positions {
            if let Some(mut tile) = self.grid.remove_tile(pos)
                && let Tile::Leaf(leaf) = &mut tile.tile
                && leaf.root
            {
                // Replace root with a new seed
                self.grid.set_tile(pos, Tile::Seed(leaf.kind));
            }
        }

        true
    }

    pub fn collect(&mut self, target: vec2<ICoord>) {
        let Some(tile) = self.grid.get_tile(target) else {
            return;
        };
        log::debug!("collect {}: {:?}", target, tile.tile);

        if tile.tile.is_collectable() {
            let mut tile = self.grid.remove_tile(target).unwrap();
            match &mut tile.tile {
                Tile::Water(lifetime) => *lifetime = self.config.water_lifetime,
                _ => {}
            }
            self.inventory_add(tile.tile, 1);
        }
    }

    pub fn inventory_add(&mut self, tile: Tile, count: usize) {
        match self.inventory.iter_mut().find(|(t, _)| *t == tile) {
            Some((_, available)) => *available += count,
            None => self.inventory.push((tile, count)),
        }
    }
}
