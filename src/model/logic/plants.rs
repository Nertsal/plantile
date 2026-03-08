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
        let options: Vec<_> = [vec2(-1, 0), vec2(0, 1), vec2(1, 0), vec2(0, -1)]
            .iter()
            .copied()
            .map(|delta| plant.pos + delta)
            .filter(|&pos| can_grow_into(pos, &self.grid))
            .collect();

        let split_chance = SPLIT_CHANCE as f64;
        let (grow_left, grow_right) = if rng.gen_bool(split_chance) {
            // Split
            let mut growth = options.choose_multiple(&mut rng, 2);
            (growth.next().copied(), growth.next().copied())
        } else {
            (options.choose(&mut rng).copied(), None)
        };

        // Spawn new plants
        let kind = leaf.kind;
        if let Some(grow) = grow_left {
            self.grid.set_tile(grow, Tile::Leaf(Leaf::new(kind)));
        }
        if let Some(grow) = grow_right {
            self.grid.set_tile(grow, Tile::Leaf(Leaf::new(kind)));
        }
    }
}

pub fn aabb_contains(aabb: Aabb2<ICoord>, pos: vec2<ICoord>) -> bool {
    aabb.min.x <= pos.x && aabb.min.y <= pos.y && aabb.max.x >= pos.x && aabb.max.y >= pos.y
}

pub fn manhattan_distance(a: vec2<ICoord>, b: vec2<ICoord>) -> ICoord {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

pub fn can_grow_into(pos: vec2<ICoord>, grid: &Grid) -> bool {
    grid.get_tile(pos).is_none()
}
