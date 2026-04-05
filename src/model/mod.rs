pub mod drone;
pub mod grid;
pub mod logic;
pub mod tiles;

pub use self::{drone::*, grid::*, tiles::*};

use crate::prelude::*;

pub type ICoord = i32;
pub type FCoord = R32;
pub type Time = R32;
pub type Money = i32;
pub type Id = usize;

pub const INVENTORY_MAX_SIZE: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    Sfx(vec2<ICoord>, GameSfx),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameSfx {
    TileBuild,
    RockSpawn,

    SeedTakeEnergy,
    PlantGrowth,
    PlantHarvest,
    WaterSpawn,
    WaterConsume,
    WaterSprinkle,
    WaterEvaporate,

    BugSpawn,
    BugMove,
    BugEat,
    BugPoop,
    PoopConsume,
    PoopDespawn,
}

pub enum ActionId {
    Drone,
    Queued(usize),
}

pub struct Model {
    pub context: Context,
    pub camera: Camera2d,
    /// Data used to convert between grid and world coordinates.
    pub grid_visual: GridVisual,
    pub config: Config,
    pub unlocked_shop: Vec<TileKind>,

    pub simulation_time: Time,
    pub next_id: Id,
    /// Actual logic data.
    pub grid: Grid,
    pub money: Money,
    pub drone: Drone,
    pub queued_actions: VecDeque<DroneTarget>,
    pub inventory: LinearMap<TileKind, usize>,

    pub events: Vec<GameEvent>,
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

            simulation_time: Time::ZERO,
            next_id: 1,
            grid: Grid::new(),
            money: 0,
            drone: Drone {
                position: vec2::ZERO,
                velocity: vec2::ZERO,
                target: None,
                action_progress: R32::ZERO,
            },
            queued_actions: VecDeque::new(),
            inventory: [(TileKind::Seed(Seed::new(PlantKind::TypeA)), 1)]
                .into_iter()
                .collect(),

            events: Vec::new(),

            config,
            context,
        }
    }
}
