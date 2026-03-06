mod logic;

use crate::prelude::*;

pub type ICoord = i32;
pub type FCoord = R32;
pub type Time = R32;

pub struct Model {
    pub camera: Camera2d,
    /// Data used to convert between grid and world coordinates.
    pub grid_visual: GridVisual,

    /// Actual logic data.
    pub grid: Grid,
}

pub enum PlantKind {
    /// Early plant (starter)
    /// - Grows uncontrollably
    /// - blocks a lot of space
    /// - breaks wire
    /// - easy to get eaten by bugs
    Early,
}

pub struct Plant {
    /// Time until the plant attempts to grow.
    pub growth_timer: Time,
    /// Position of the plant's root which is permanent.
    pub root: vec2<ICoord>,
    /// The stem connects the root with the leaves.
    pub stem: Vec<vec2<ICoord>>,
    /// Leaves let the plant grow.
    /// If there are no leaves but some stem tiles, the plant can no longer grow.
    /// If there are also no stem tiles, the plant can grow from the root (this is assumed to be the initial state).
    pub leaves: Vec<vec2<ICoord>>,
    pub kind: PlantKind,
}

impl Plant {
    pub fn new(position: vec2<ICoord>, kind: PlantKind) -> Self {
        Self {
            growth_timer: r32(1.0),
            root: position,
            stem: vec![],
            leaves: vec![],
            kind,
        }
    }
}

pub struct Grid {
    pub plants: Vec<Plant>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            plants: vec![Plant::new(vec2(0, 1), PlantKind::Early)],
        }
    }
}

pub struct GridVisual {
    /// Position of the (0, 0) point in the world.
    pub center: vec2<FCoord>,
    pub tile_size: vec2<FCoord>,
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

impl Model {
    pub fn new() -> Self {
        Self {
            camera: Camera2d {
                center: vec2(0.5, 10.0),
                rotation: Angle::ZERO,
                fov: Camera2dFov::Vertical(30.0),
            },
            grid_visual: GridVisual {
                center: vec2::ZERO,
                tile_size: vec2(1.0, 1.0).as_r32(),
            },

            grid: Grid::new(),
        }
    }
}
