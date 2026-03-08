mod actions;
mod plants;

pub use self::{actions::*, plants::*};

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: Time) {
        // Update tiles
        let update_order: Vec<vec2<ICoord>> = self.grid.all_positions().collect();
        for pos in update_order {
            let Some(tile) = self.grid.get_tile(pos) else {
                continue;
            };
            match &tile.tile {
                Tile::Leaf(_) => self.update_plant(tile.pos, delta_time),
                Tile::Light => {}
            }
        }
    }
}

fn get_all_connected(
    grid: &Grid,
    start: vec2<ICoord>,
    mut condition: impl FnMut(Positioned<&Tile>) -> bool,
) -> Vec<vec2<ICoord>> {
    let mut connected = vec![start];
    let mut to_check = vec![start];

    while let Some(pos) = to_check.pop() {
        for tile in grid.get_neighbors(pos) {
            if !connected.contains(&tile.pos) && condition(tile) {
                connected.push(tile.pos);
                to_check.push(tile.pos);
            }
        }
    }

    connected
}
