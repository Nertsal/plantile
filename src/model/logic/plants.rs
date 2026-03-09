use super::*;

const MAX_SIZE: usize = 15;
const SPLIT_CHANCE: f32 = 0.1;

impl Model {
    pub fn update_plant(&mut self, position: vec2<ICoord>, delta_time: Time) {
        let Some(mut plant) = self.grid.get_tile_mut(position) else {
            return;
        };
        let Tile::Leaf(leaf) = &mut plant.tile else {
            return;
        };

        let mut rng = thread_rng();

        // Update growth timer
        let mut grow = false;
        if let Some(timer) = &mut leaf.growth_timer {
            *timer -= delta_time;
            if *timer <= Time::ZERO {
                // Attempt to grow
                grow = true;
                leaf.growth_timer = None;
            }
        }

        if !grow {
            return;
        }

        // Grow
        let Some(plant) = self.grid.get_tile(position) else {
            return;
        };
        let Tile::Leaf(leaf) = &plant.tile else {
            return;
        };
        if get_all_connected(&self.grid, plant.pos, |tile| {
            matches!(tile.tile, Tile::Leaf(_))
        })
        .len()
            >= MAX_SIZE
        {
            // Over max size
            return;
        }

        // Grow
        let lights: Vec<vec2<ICoord>> = self
            .grid
            .all_tiles()
            .filter(|tile| matches!(tile.tile, Tile::Light(true)))
            .map(|tile| tile.pos)
            .collect();
        let options: Vec<_> = [vec2(-1, 0), vec2(0, 1), vec2(1, 0), vec2(0, -1)]
            .iter()
            .copied()
            .map(|delta| plant.pos + delta)
            .filter(|&pos| can_grow_into(pos, &self.grid))
            .map(|pos| {
                let light_d = lights
                    .iter()
                    .map(|light| manhattan_distance(pos, *light))
                    .min()
                    .unwrap_or(self.config.light_radius)
                    .min(self.config.light_radius);
                let plant_density = density_around(&self.grid, pos);
                let weight = 1.0
                    + 0.5 * (((self.config.light_radius - light_d) as f32) * 0.7).exp()
                    + 20.0 * plant_density.recip().powi(2);
                (pos, weight)
            })
            .collect();

        let weight = |(_, w): &(vec2<ICoord>, f32)| *w;
        let value = |(v, _)| v;

        let split_chance = SPLIT_CHANCE as f64;
        let (grow_left, grow_right) = if rng.gen_bool(split_chance) {
            // Split
            let mut growth = options
                .choose_multiple_weighted(&mut rng, 2, weight)
                .into_iter()
                .flatten();
            (
                growth.next().copied().map(value),
                growth.next().copied().map(value),
            )
        } else {
            (
                options
                    .choose_weighted(&mut rng, weight)
                    .ok()
                    .copied()
                    .map(value),
                None,
            )
        };

        // Spawn new plants
        let kind = leaf.kind;
        let growth_time = self.config.plant_growth_time;
        if let Some(grow) = grow_left {
            self.grid
                .set_tile(grow, Tile::Leaf(Leaf::new(kind, growth_time)));
        }
        if let Some(grow) = grow_right {
            self.grid
                .set_tile(grow, Tile::Leaf(Leaf::new(kind, growth_time)));
        }
    }
}

pub fn can_grow_into(pos: vec2<ICoord>, grid: &Grid) -> bool {
    grid.get_tile(pos).is_none()
}

fn density_around(grid: &Grid, pos: vec2<ICoord>) -> f32 {
    let range = 1;
    let leaves = (-range..=range)
        .flat_map(|dx| {
            (-range..=range).flat_map(move |dy| {
                let pos = pos + vec2(dx, dy);
                grid.get_tile(pos)
            })
        })
        .filter(|tile| matches!(tile.tile, Tile::Leaf(_)))
        .count();
    let area = ((range * 2 + 1) as f32).sqr();
    leaves as f32 / area
}
