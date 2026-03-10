pub mod logic;

use crate::prelude::*;

pub type ICoord = i32;
pub type FCoord = R32;
pub type Time = R32;
pub type Money = i32;
pub type Id = usize;

pub const INVENTORY_MAX_SIZE: usize = 10;

pub struct Model {
    pub context: Context,
    pub camera: Camera2d,
    /// Data used to convert between grid and world coordinates.
    pub grid_visual: GridVisual,
    pub config: Config,
    pub unlocked_shop: Vec<TileKind>,

    pub next_id: Id,
    /// Actual logic data.
    pub grid: Grid,
    pub money: Money,
    pub drone: Drone,
    pub inventory: Vec<(TileKind, usize)>,
}

#[derive(Debug, Clone)]
pub enum DroneTarget {
    MoveTo(vec2<ICoord>),
    Interact(vec2<ICoord>, DroneAction),
    PlaceTile(vec2<ICoord>, TileKind),
    BuyTile(vec2<ICoord>, TileKind),
    KillBug(Id),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DroneAction {
    CutPlant,
    Collect,
    PlaceTile,
    KillBug,
}

#[derive(Debug)]
pub struct Drone {
    pub position: vec2<FCoord>,
    pub velocity: vec2<FCoord>,
    pub target: DroneTarget,
    pub action_progress: R32,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Leaf {
    /// Time until the plant attempts to grow.
    pub growth_timer: Option<Time>,
    pub kind: PlantKind,
    pub connections: Connections,
}

impl Leaf {
    pub fn new(kind: PlantKind) -> Self {
        Self {
            growth_timer: Some(R32::ONE),
            kind,
            connections: Connections::new(),
        }
    }

    pub fn connected(mut self, side: vec2<ICoord>) -> Self {
        self.connections.set(side, Some(()));
        self
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
    Hungry {
        hunger: usize,
        eating_timer: Lifetime,
    },
    Pooping(Lifetime),
    Chilling {
        time: Time,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Cutter {
    pub powered: bool,
    pub cooldown: Lifetime,
}

impl Default for Cutter {
    fn default() -> Self {
        Self {
            powered: false,
            cooldown: Lifetime::new(R32::ONE),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lifetime {
    pub remaining: Time,
    pub max: Time,
}

impl Lifetime {
    pub fn new(max: Time) -> Self {
        Self {
            remaining: max,
            max,
        }
    }

    pub fn ratio(&self) -> Time {
        if self.max == Time::ZERO {
            return Time::ZERO;
        }
        self.remaining / self.max
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tile {
    pub state: TileState,
    pub kind: TileKind,
}

impl Tile {
    pub fn new(kind: TileKind) -> Self {
        Self {
            state: TileState::Spawning(Lifetime::new(R32::ONE)),
            kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TileState {
    Spawning(Lifetime),
    Idle,
    Despawning(Lifetime),
    /// Similar to [`Spawning`] but different semantics.
    Transforming(Lifetime),
    Moving {
        timer: Lifetime,
        delta: vec2<ICoord>,
    },
}

impl TileState {
    pub fn interactive(&self) -> bool {
        matches!(self, TileState::Idle)
    }

    pub fn despawn(&mut self) {
        if !matches!(self, Self::Despawning(_)) {
            *self = Self::Despawning(Lifetime::new(Time::ONE));
        }
    }

    pub fn transform(&mut self) {
        *self = Self::Transforming(Lifetime::new(Time::ONE));
    }

    pub fn moving(&mut self, delta: vec2<ICoord>) {
        *self = Self::Moving {
            timer: Lifetime::new(Time::ONE),
            delta,
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TileKind {
    /// Not a real tile, but a placeholder to prevent stuff from happening.
    /// Used by animations and such.
    GhostBlock,
    Seed(PlantKind),
    Leaf(Leaf),
    Light(bool),
    Soil(SoilState),
    Water(Lifetime),
    Bug(Bug),
    Poop(Lifetime),
    Power,
    Wire(bool),
    Drainer,
    Cutter(Cutter),
    Pipe(bool),
    Sprinkler(bool),
    Rock,
}

impl TileKind {
    pub fn name(&self) -> &'static str {
        match self {
            TileKind::GhostBlock => "Huh?",
            TileKind::Seed(kind) => match kind {
                PlantKind::TypeA => "Seed (A)",
                PlantKind::TypeB => "Seed (B)",
                PlantKind::TypeC => "Seed (C)",
                PlantKind::TypeD => "Seed (D)",
            },
            TileKind::Leaf(leaf) => match leaf.kind {
                PlantKind::TypeA => "Leaf (A)",
                PlantKind::TypeB => "Leaf (B)",
                PlantKind::TypeC => "Leaf (C)",
                PlantKind::TypeD => "Leaf (D)",
            },
            TileKind::Light(_) => "Light",
            TileKind::Soil(state) => match state {
                SoilState::Dry => "Dry Soil",
                SoilState::Watered => "Soil",
                SoilState::Rich => "Rich Soil",
            },
            TileKind::Water(_) => "Water",
            TileKind::Bug(_) => "Bug",
            TileKind::Poop(_) => "Poop",
            TileKind::Power => "Power",
            TileKind::Wire(_) => "Wire",
            TileKind::Drainer => "Drainer",
            TileKind::Cutter(_) => "Cutter",
            TileKind::Pipe(_) => "Pipe",
            TileKind::Sprinkler(_) => "Sprinkler",
            TileKind::Rock => "Rock",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TileKind::GhostBlock => "You are not supposed to see this",
            TileKind::Seed(kind) => match kind {
                PlantKind::TypeA => "Grows from Dry Soil",
                PlantKind::TypeB => "Grows from Soil",
                PlantKind::TypeC => "Grows from Water",
                PlantKind::TypeD => "Grows from Rich Soil",
            },
            TileKind::Leaf(leaf) => match leaf.kind {
                PlantKind::TypeA => "Sells for 3g",
                PlantKind::TypeB => "Sells for 10g",
                PlantKind::TypeC => "Sells for 50g",
                PlantKind::TypeD => "Sells for 30g",
            },
            TileKind::Light(_) => "Plants grow within range\nrequires Power",
            TileKind::Soil(state) => match state {
                SoilState::Dry => "",
                SoilState::Watered => "",
                SoilState::Rich => "",
            },
            TileKind::Water(_) => "",
            TileKind::Bug(_) => "Eats Plants and produces Poop",
            TileKind::Poop(_) => "Can be used to nourish the soil",
            TileKind::Power => "Provides power to connected tiles",
            TileKind::Wire(_) => "",
            TileKind::Drainer => {
                "Collects Water within range to your inventory or to connected Sprinklers"
            }
            TileKind::Cutter(_) => "Automatically cuts adjacent Plants\nrequires Power",
            TileKind::Pipe(_) => "",
            TileKind::Sprinkler(_) => "Ejects water on adjacent tiles",
            TileKind::Rock => "",
        }
    }

    pub fn update_order(&self) -> i32 {
        match self {
            TileKind::Drainer => 100, // After soil and seed so it takes water first
            _ => 0,
        }
    }

    pub fn is_collectable(&self) -> bool {
        matches!(
            self,
            TileKind::Seed(_)
                | TileKind::Light(_)
                | TileKind::Soil(_)
                | TileKind::Water(_)
                | TileKind::Poop(_)
                | TileKind::Power
                | TileKind::Wire(_)
                | TileKind::Pipe(_)
                | TileKind::Cutter(_)
                | TileKind::Sprinkler(_)
                | TileKind::Rock
                | TileKind::Drainer
        )
    }

    pub fn transmits_power(&self) -> bool {
        matches!(
            self,
            TileKind::Power | TileKind::Wire(_) | TileKind::Light(_) | TileKind::Cutter(_)
        )
    }

    pub fn is_piping(&self) -> bool {
        matches!(
            self,
            TileKind::Drainer | TileKind::Pipe(_) | TileKind::Sprinkler(_)
        )
    }

    pub fn action_progress(&self) -> Option<R32> {
        match self {
            TileKind::Leaf(leaf) => leaf.growth_timer.map(|t| R32::ONE - t),
            TileKind::Water(lifetime) | TileKind::Poop(lifetime) => Some(lifetime.ratio()),
            TileKind::Cutter(cutter) => Some(cutter.cooldown.ratio()),
            TileKind::Bug(bug) => match &bug.state {
                BugState::Hungry { eating_timer, .. } => {
                    let t = eating_timer.ratio();
                    (t < Time::ONE).then_some(t)
                }
                BugState::Pooping(timer) => Some(timer.ratio()),
                _ => None,
            },
            _ => None,
        }
    }
}

pub struct Grid {
    pub bounds: Aabb2<ICoord>,
    pub tiles: HashMap<vec2<ICoord>, Tile>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            bounds: Aabb2::point(vec2(0, 5)).extend_symmetric(vec2(30, 15)),
            tiles: hashmap! {
                vec2(0, 0) => Tile::new(TileKind::Soil(SoilState::Dry)),
                vec2(0, 9) => Tile::new(TileKind::Light(false)),
                vec2(0, 10) => Tile::new(TileKind::Wire(false)),
                vec2(0, 11) => Tile::new(TileKind::Wire(false)),
                vec2(-1, 11) => Tile::new(TileKind::Power),
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
            matches!(light.tile.kind, TileKind::Light(_))
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

impl Model {
    pub fn new(context: Context, config: Config) -> Self {
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
            money: 0,
            drone: Drone {
                position: vec2::ZERO,
                velocity: vec2::ZERO,
                target: DroneTarget::MoveTo(vec2::ZERO),
                action_progress: R32::ZERO,
            },
            inventory: vec![(TileKind::Seed(PlantKind::TypeA), 1)],

            config,
            context,
        }
    }
}
