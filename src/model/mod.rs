pub mod logic;

use crate::prelude::*;

pub type ICoord = i32;
pub type FCoord = R32;
pub type Time = R32;
pub type Money = i32;
pub type Id = usize;

pub struct Model {
    pub camera: Camera2d,
    /// Data used to convert between grid and world coordinates.
    pub grid_visual: GridVisual,
    pub config: Config,
    pub unlocked_shop: Vec<Tile>,

    pub next_id: Id,
    /// Actual logic data.
    pub grid: Grid,
    pub money: Money,
    pub drone: Drone,
    pub inventory: Vec<(Tile, usize)>,
}

#[derive(Debug, Clone)]
pub enum DroneTarget {
    MoveTo(vec2<ICoord>),
    Interact(vec2<ICoord>, DroneAction),
    PlaceTile(vec2<ICoord>, Tile),
    BuyTile(vec2<ICoord>, Tile),
    KillBug(Id),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DroneAction {
    CutPlant,
    Collect,
}

#[derive(Debug)]
pub struct Drone {
    pub position: vec2<FCoord>,
    pub velocity: vec2<FCoord>,
    pub target: DroneTarget,
    pub action_progress: R32,
}

impl Drone {
    pub const ACCELERATION: f32 = 20.0;
    pub const DECELERATION: f32 = 10.0;
    pub const MAX_SPEED: f32 = 20.0;
    pub const REACH: f32 = 0.5;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PlantKind {
    /// Starter plant
    /// - Grows uncontrollably
    /// - blocks a lot of space
    /// - breaks wire
    /// - easy to get eaten by bugs
    TypeA,
    TypeB,
    TypeC,
    TypeD,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Leaf {
    /// Time until the plant attempts to grow.
    pub growth_timer: Option<Time>,
    pub root: bool,
    pub kind: PlantKind,
}

impl Leaf {
    pub fn new(kind: PlantKind) -> Self {
        Self {
            growth_timer: Some(R32::ONE),
            root: false,
            kind,
        }
    }

    pub fn root(self) -> Self {
        Self { root: true, ..self }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Positioned<T> {
    pub pos: vec2<ICoord>,
    pub tile: T,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SoilState {
    Dry,
    Watered,
    Rich,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bug {
    pub id: Id,
    pub state: BugState,
    pub move_timer: Time,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BugState {
    Hungry { hunger: usize, eating_timer: Time },
    Pooping(Time),
    Chilling { time: Time },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Cutter {
    pub powered: bool,
    pub cooldown: Time,
}

impl Default for Cutter {
    fn default() -> Self {
        Self {
            powered: false,
            cooldown: R32::ONE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Tile {
    Seed(PlantKind),
    Leaf(Leaf),
    Light(bool),
    Soil(SoilState),
    Water(Time),
    Bug(Bug),
    Poop(Time),
    Power,
    Wire(bool),
    Drainer,
    Cutter(Cutter),
    Pipe(bool),
    Sprinkler(bool),
}

impl Tile {
    pub fn name(&self) -> &'static str {
        match self {
            Tile::Seed(kind) => match kind {
                PlantKind::TypeA => "Seed (A)",
                PlantKind::TypeB => "Seed (B)",
                PlantKind::TypeC => "Seed (C)",
                PlantKind::TypeD => "Seed (D)",
            },
            Tile::Leaf(leaf) => match leaf.kind {
                PlantKind::TypeA => "Leaf (A)",
                PlantKind::TypeB => "Leaf (B)",
                PlantKind::TypeC => "Leaf (C)",
                PlantKind::TypeD => "Leaf (D)",
            },
            Tile::Light(_) => "Light",
            Tile::Soil(state) => match state {
                SoilState::Dry => "Dry Soil",
                SoilState::Watered => "Soil",
                SoilState::Rich => "Rich Soil",
            },
            Tile::Water(_) => "Water",
            Tile::Bug(_) => "Bug",
            Tile::Poop(_) => "Poop",
            Tile::Power => "Power",
            Tile::Wire(_) => "Wire",
            Tile::Drainer => "Drainer",
            Tile::Cutter(_) => "Cutter",
            Tile::Pipe(_) => "Pipe",
            Tile::Sprinkler(_) => "Sprinkler",
        }
    }

    pub fn update_order(&self) -> i32 {
        match self {
            Tile::Drainer => 100, // After soil and seed so it takes water first
            _ => 0,
        }
    }

    pub fn is_collectable(&self) -> bool {
        matches!(
            self,
            Tile::Seed(_)
                | Tile::Light(_)
                | Tile::Soil(_)
                | Tile::Water(_)
                | Tile::Poop(_)
                | Tile::Power
                | Tile::Wire(_)
        )
    }

    pub fn transmits_power(&self) -> bool {
        matches!(
            self,
            Tile::Power | Tile::Wire(_) | Tile::Light(_) | Tile::Cutter(_)
        )
    }

    pub fn is_piping(&self) -> bool {
        matches!(self, Tile::Drainer | Tile::Pipe(_) | Tile::Sprinkler(_))
    }
}

pub struct Grid {
    pub tiles: HashMap<vec2<ICoord>, Tile>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            tiles: hashmap! {
                vec2(0, 0) => Tile::Soil(SoilState::Dry),
                vec2(0, 9) => Tile::Light(false),
                vec2(0, 10) => Tile::Wire(false),
                vec2(0, 11) => Tile::Wire(false),
                vec2(-1, 11) => Tile::Power,
            },
        }
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
        offsets.into_iter().map(move |offset| Positioned {
            pos: pos + offset,
            tile: self.get_tile(pos + offset).map(|tile| tile.tile),
        })
    }

    pub fn get_neighbors(&self, pos: vec2<ICoord>) -> impl Iterator<Item = Positioned<&Tile>> {
        let offsets = [vec2(-1, 0), vec2(0, -1), vec2(1, 0), vec2(0, 1)];
        offsets
            .into_iter()
            .filter_map(move |offset| self.get_tile(pos + offset))
    }

    pub fn is_tile_lit(&self, pos: vec2<ICoord>, config: &Config) -> bool {
        self.all_tiles()
            .any(|light| logic::manhattan_distance(pos, light.pos) <= config.light_radius)
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

impl Model {
    pub fn new(config: Config) -> Self {
        Self {
            camera: Camera2d {
                center: vec2(0.5, 5.0),
                rotation: Angle::ZERO,
                fov: Camera2dFov::Vertical(15.0),
            },
            grid_visual: GridVisual {
                center: vec2::ZERO,
                tile_size: vec2(1.0, 1.0).as_r32(),
                tile_margin: vec2(0.0, 0.0).as_r32(),
            },
            unlocked_shop: Vec::new(),

            next_id: 1,
            grid: Grid::new(),
            money: 10000000,
            drone: Drone {
                position: vec2::ZERO,
                velocity: vec2::ZERO,
                target: DroneTarget::MoveTo(vec2::ZERO),
                action_progress: R32::ZERO,
            },
            inventory: vec![(Tile::Seed(PlantKind::TypeA), 1)],

            config,
        }
    }
}
