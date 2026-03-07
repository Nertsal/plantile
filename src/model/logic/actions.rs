use super::*;

impl Model {
    pub fn cut_plant(&mut self, plant_idx: usize) {
        let Some(plant) = &mut self.grid.plants.get_mut(plant_idx) else {
            return;
        };

        // Earn money
        let size = plant.stem.len() + plant.leaves.len();
        self.money += size as Money;

        // Remove stem and leaves
        plant.stem.clear();
        plant.leaves.clear();
    }

    /// Attempt to plant a seed of a specific kind at the given position.
    /// Returns `true` if planted.
    pub fn plant_seed(&mut self, target: vec2<ICoord>, kind: PlantKind) -> bool {
        log::debug!("plant");
        if self.grid.plants.iter().any(|plant| {
            plant.root == target || plant.stem.contains(&target) || plant.leaves.contains(&target)
        }) {
            return false;
        }

        self.grid.plants.push(Plant::new(target, kind));
        true
    }
}
