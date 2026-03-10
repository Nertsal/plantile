use super::*;

const SPLIT_CHANCE: f32 = 0.1;

impl Model {
    pub fn update_plant(&mut self, position: vec2<ICoord>, delta_time: Time) {
        let is_lit = self.grid.is_tile_lit(position, &self.config);

        macro_rules! let_leaf {
            (let $plant:ident, $leaf:ident) => {
                let Some($plant) = self.grid.get_tile(position) else {
                    return;
                };
                let TileKind::Leaf($leaf) = &$plant.tile.kind else {
                    return;
                };
            };
            (let mut $plant:ident, $leaf:ident) => {
                let Some($plant) = self.grid.get_tile_mut(position) else {
                    return;
                };
                let TileKind::Leaf($leaf) = &mut $plant.tile.kind else {
                    return;
                };
            };
        }

        // Update connections
        let_leaf!(let plant, leaf);
        let mut connections = leaf.connections.clone();
        for delta in Connections::NEIGHBORS {
            if connections.get(delta).is_some()
                && self
                    .grid
                    .get_tile(position + delta)
                    .is_none_or(|tile| !matches!(tile.tile.kind, TileKind::Leaf(_)))
            {
                // Connection dropped
                connections.set(delta, None);
            }
        }
        let connect_count = connections.get_connections(position).count();

        let mut rng = thread_rng();
        let plant_config = &self.config.plants[&leaf.kind];

        // Update growth timer
        let_leaf!(let mut plant, leaf);
        leaf.connections = connections;
        if leaf.growth_timer.is_none() && (connect_count == 0 || (!leaf.root && connect_count <= 1))
        {
            leaf.growth_timer = Some(R32::ONE);
        }

        let mut grow = false;
        if let Some(timer) = &mut leaf.growth_timer {
            let growth_time = if is_lit {
                plant_config.growth_time
            } else {
                plant_config.growth_time_dark
            };
            *timer -= delta_time / growth_time;
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
        let_leaf!(let plant, leaf);
        if get_all_connected(&self.grid, plant.pos, |tile| {
            if let TileKind::Leaf(other) = tile.tile
                && leaf.kind == other.kind
            {
                true
            } else {
                false
            }
        })
        .len()
            >= plant_config.max_size
        {
            // Over max size
            return;
        }

        // Grow
        let lights: Vec<vec2<ICoord>> = self
            .grid
            .all_tiles()
            .filter(|tile| matches!(tile.tile.kind, TileKind::Light(true)))
            .map(|tile| tile.pos)
            .collect();

        let options: Vec<_> = self
            .grid
            .get_neighbors_all(plant.pos)
            .filter_map(|tile| tile.tile.is_none().then_some(tile.pos))
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
        if let Some(grow) = grow_left {
            let mut leaf = Leaf::new(kind);
            leaf.connections.set(position - grow, Some(()));
            // TODO: animation
            self.grid.set_tile(grow, Tile::new(TileKind::Leaf(leaf)));
            self.context.sfx.play(&self.context.assets.sounds.grow);
        }
        if let Some(grow) = grow_right {
            let mut leaf = Leaf::new(kind);
            leaf.connections.set(position - grow, Some(()));
            // TODO: animation
            self.grid.set_tile(grow, Tile::new(TileKind::Leaf(leaf)));
        }

        // Connect
        let_leaf!(let mut plant, leaf);
        if let Some(grow) = grow_left {
            leaf.connections.set(grow - position, Some(()));
        }
        if let Some(grow) = grow_right {
            leaf.connections.set(grow - position, Some(()));
        }
    }
}

pub fn can_grow_into(pos: vec2<ICoord>, grid: &Grid) -> bool {
    match grid.get_tile(pos) {
        Some(tile) => matches!(tile.tile.kind, TileKind::Wire(_) | TileKind::Pipe(_)),
        None => true,
    }
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
        .filter(|tile| matches!(tile.tile.kind, TileKind::Leaf(_)))
        .count();
    let area = ((range * 2 + 1) as f32).sqr();
    leaves as f32 / area
}
