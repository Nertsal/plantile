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
            // Tile::Bug(bug_id) => self.drone.target = DroneTarget::KillBug(bug_id),
            _ => DroneTarget::Interact(target),
        };
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
                leaf.growth_timer = Some(r32(1.0));
                self.grid.set_tile(pos, tile.tile);
            }
        }

        true
    }

    /// Attempt to plant a seed of a specific kind at the given position.
    /// Returns `true` if planted.
    pub fn plant_seed(&mut self, target: vec2<ICoord>, kind: PlantKind) -> bool {
        log::debug!("plant at {}: {:?}", target, kind);
        if self.grid.get_tile(target).is_some() {
            // Occupied tile
            return false;
        }

        self.grid.set_tile(target, Tile::Leaf(Leaf::new(kind)));
        true
    }
}
