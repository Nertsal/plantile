mod actions;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: Time) {
        let mut rng = thread_rng();

        // Update plants
        for plant in &mut self.grid.plants {
            if plant.growth_timer > Time::ZERO {
                plant.growth_timer -= delta_time;
            } else {
                if plant.stem.len() > 10 {
                    // Stop growing at 10
                    continue;
                }

                // Attempt to grow
                plant.growth_timer += r32(0.5); // TODO: configurable somewhere somehow for different kinds of plants
                if plant.leaves.is_empty() {
                    if plant.stem.is_empty() {
                        // Grow from the root
                        // TODO: check for obstruction
                        plant.leaves.push(plant.root + vec2(0, 1));
                    }
                } else {
                    // Grow from the leaves
                    for leaf in &mut plant.leaves {
                        // TODO: check for obstruction
                        // TODO: splitting.
                        // To split and to check obstruction you'd need to refactor the code to avoid mutably borrowing the plants and leaves while calculating the growth.
                        // In short, for each plant that needs to grow, you need to first calculate the growth (the change) that happens using immutable borrows, and then apply that change to the data.
                        let options =
                            [vec2(-1, 1), vec2(0, 1), vec2(1, 1)].map(|delta| *leaf + delta);
                        if let Some(&growth) = options.choose(&mut rng) {
                            plant.stem.push(*leaf);
                            *leaf = growth;
                        }
                    }
                }
            }
        }
    }
}
