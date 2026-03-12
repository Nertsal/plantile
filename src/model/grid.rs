use super::*;

pub struct Grid {
    pub bounds: Aabb2<ICoord>,
    pub tiles: HashMap<vec2<ICoord>, Tile>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            bounds: Aabb2::point(vec2(0, 5)).extend_symmetric(vec2(30, 15)),
            tiles: hashmap! {
                vec2(0, 1) => Tile::new(TileKind::Soil(SoilState::Dry)),
                vec2(0, 7) => Tile::new(TileKind::Light(false)),
                vec2(0, 8) => Tile::new(TileKind::Wire(false)),
                vec2(-1, 8) => Tile::new(TileKind::Power),
            },
        }
    }

    pub fn in_bounds(&self, pos: vec2<ICoord>) -> bool {
        self.bounds.min.x <= pos.x
            && pos.x <= self.bounds.max.x
            && self.bounds.min.y <= pos.y
            && pos.y <= self.bounds.max.y
    }

    pub fn all_tiles(&self) -> impl Iterator<Item = Positioned<&Tile>> {
        self.tiles
            .iter()
            .map(|(pos, tile)| Positioned { pos: *pos, tile })
    }

    pub fn all_positions(&self) -> impl Iterator<Item = vec2<ICoord>> {
        self.tiles.keys().copied()
    }

    pub fn get_tile(&self, pos: vec2<ICoord>) -> Option<Positioned<&Tile>> {
        self.tiles.get(&pos).map(|tile| Positioned { pos, tile })
    }

    pub fn get_tile_mut(&mut self, pos: vec2<ICoord>) -> Option<Positioned<&mut Tile>> {
        self.tiles
            .get_mut(&pos)
            .map(|tile| Positioned { pos, tile })
    }

    pub fn remove_tile(&mut self, pos: vec2<ICoord>) -> Option<Positioned<Tile>> {
        self.tiles.remove(&pos).map(|tile| Positioned { pos, tile })
    }

    pub fn set_tile(&mut self, pos: vec2<ICoord>, tile: Tile) -> Option<Positioned<Tile>> {
        self.tiles
            .insert(pos, tile)
            .map(|tile| Positioned { pos, tile })
    }

    pub fn get_neighbors_all(
        &self,
        pos: vec2<ICoord>,
    ) -> impl Iterator<Item = Positioned<Option<&Tile>>> {
        let offsets = [vec2(-1, 0), vec2(0, -1), vec2(1, 0), vec2(0, 1)];
        offsets
            .into_iter()
            .map(move |offset| Positioned {
                pos: pos + offset,
                tile: self.get_tile(pos + offset).map(|tile| tile.tile),
            })
            .filter(|tile| self.in_bounds(tile.pos))
    }

    pub fn get_neighbors(&self, pos: vec2<ICoord>) -> impl Iterator<Item = Positioned<&Tile>> {
        let offsets = [vec2(-1, 0), vec2(0, -1), vec2(1, 0), vec2(0, 1)];
        offsets
            .into_iter()
            .filter_map(move |offset| self.get_tile(pos + offset))
    }

    pub fn is_tile_lit(&self, pos: vec2<ICoord>, config: &Config) -> bool {
        self.all_tiles().any(|light| {
            matches!(light.tile.kind, TileKind::Light(true))
                && logic::manhattan_distance(pos, light.pos) <= config.light_radius
        })
    }
}

pub struct GridVisual {
    /// Position of the (0, 0) point in the world.
    pub center: vec2<FCoord>,
    /// Full size of the tile.
    pub tile_size: vec2<FCoord>,
    /// Margin applied to make the tile visually smaller and leave space in-between tiles.
    pub tile_margin: vec2<FCoord>,
}

impl GridVisual {
    pub fn grid_to_world(&self, grid: vec2<ICoord>) -> vec2<FCoord> {
        grid.as_r32() * self.tile_size + self.center
    }

    /// World coordinates AABB of the tile.
    pub fn tile_bounds(&self, grid: vec2<ICoord>) -> Aabb2<FCoord> {
        let min = self.grid_to_world(grid);
        Aabb2 {
            min,
            max: min + self.tile_size,
        }
        .extend_symmetric(-self.tile_margin)
    }

    /// World coordinates AABB of the multiple tiles.
    pub fn multitile_bounds(&self, grid: Aabb2<ICoord>) -> Aabb2<FCoord> {
        let min = self.grid_to_world(grid.min);
        let max = self.grid_to_world(grid.max) + self.tile_size;
        Aabb2 { min, max }.extend_symmetric(-self.tile_margin)
    }

    /// Calculate the grid position along with the offset from the bottom left corner of the tile.
    pub fn world_to_grid_offset(&self, world: vec2<FCoord>) -> (vec2<ICoord>, vec2<FCoord>) {
        let pos = (world - self.center) / self.tile_size;
        let grid = pos.map(|x| x.floor());
        let offset = (pos - grid) * self.tile_size;
        (grid.map(|x| x.as_f32() as ICoord), offset)
    }

    pub fn world_to_grid(&self, world: vec2<FCoord>) -> vec2<ICoord> {
        self.world_to_grid_offset(world).0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Connections<T = ()> {
    pub left: Option<T>,
    pub right: Option<T>,
    pub down: Option<T>,
    pub up: Option<T>,
}

impl Connections {
    pub const NEIGHBORS: [vec2<ICoord>; 4] = [vec2(-1, 0), vec2(0, -1), vec2(1, 0), vec2(0, 1)];
}

impl<T> Connections<T> {
    pub fn new() -> Self {
        Self {
            left: None,
            right: None,
            down: None,
            up: None,
        }
    }

    pub fn get(&self, delta: vec2<ICoord>) -> Option<&T> {
        match delta {
            vec2(-1, 0) => self.left.as_ref(),
            vec2(0, -1) => self.down.as_ref(),
            vec2(1, 0) => self.right.as_ref(),
            vec2(0, 1) => self.up.as_ref(),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, delta: vec2<ICoord>) -> Option<&mut T> {
        match delta {
            vec2(-1, 0) => self.left.as_mut(),
            vec2(0, -1) => self.down.as_mut(),
            vec2(1, 0) => self.right.as_mut(),
            vec2(0, 1) => self.up.as_mut(),
            _ => None,
        }
    }

    pub fn set(&mut self, delta: vec2<ICoord>, value: Option<T>) -> Option<T> {
        match delta {
            vec2(-1, 0) => std::mem::replace(&mut self.left, value),
            vec2(0, -1) => std::mem::replace(&mut self.down, value),
            vec2(1, 0) => std::mem::replace(&mut self.right, value),
            vec2(0, 1) => std::mem::replace(&mut self.up, value),
            _ => None,
        }
    }

    pub fn get_all(&self, position: vec2<ICoord>) -> [Positioned<Option<&T>>; 4] {
        fn mk<T>(
            position: vec2<ICoord>,
            dx: ICoord,
            dy: ICoord,
            item: Option<&T>,
        ) -> Positioned<Option<&T>> {
            Positioned {
                pos: position + vec2(dx, dy),
                tile: item,
            }
        }
        [
            mk(position, -1, 0, self.left.as_ref()),
            mk(position, 0, -1, self.down.as_ref()),
            mk(position, 1, 0, self.right.as_ref()),
            mk(position, 0, 1, self.up.as_ref()),
        ]
    }

    pub fn get_connections(&self, position: vec2<ICoord>) -> impl Iterator<Item = Positioned<&T>> {
        self.get_all(position).into_iter().filter_map(|neighbor| {
            neighbor.tile.map(|tile| Positioned {
                pos: neighbor.pos,
                tile,
            })
        })
    }
}
