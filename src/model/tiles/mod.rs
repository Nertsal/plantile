use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PlantKind {
    TypeA,
    TypeB,
    TypeC,
    TypeD,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "PlantKind")]
pub struct Seed {
    pub kind: PlantKind,
    /// How much growth we have of each speed multiplier.
    pub growth_energy: LinearMap<Time, R32>,
    pub growth_timer: Time,
}

impl From<PlantKind> for Seed {
    fn from(value: PlantKind) -> Self {
        Self::new(value)
    }
}

impl Seed {
    pub fn new(kind: PlantKind) -> Self {
        Self {
            kind,
            growth_energy: LinearMap::new(),
            growth_timer: Time::ONE,
        }
    }

    pub fn total_energy(&self) -> R32 {
        self.growth_energy
            .values()
            .copied()
            .fold(R32::ZERO, R32::add)
    }

    pub fn use_energy(&mut self, mut energy: R32) {
        for (_, remaining) in self.growth_energy.iter_mut().sorted_by_key(|(s, _)| **s) {
            if *remaining >= energy {
                *remaining -= energy;
                break;
            } else {
                energy -= *remaining;
                *remaining = R32::ZERO;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Leaf {
    /// Time until the plant attempts to grow.
    pub growth_timer: Option<Time>,
    /// Whether the leaf is currently growing.
    /// Could be stuck in a corner, out of resources, or fully grown.
    pub is_growing: bool,
    pub kind: PlantKind,
    pub connections: Connections,
}

impl Leaf {
    pub fn new(kind: PlantKind) -> Self {
        Self {
            growth_timer: Some(R32::ONE),
            is_growing: true,
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
    pub cut_timer: Lifetime,
}

impl Default for Cutter {
    fn default() -> Self {
        Self {
            powered: false,
            cut_timer: Lifetime::new(R32::ONE),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Lifetime {
    pub remaining: Time,
    pub max: Time,
}

impl Default for Lifetime {
    fn default() -> Self {
        Self::new(Time::ONE)
    }
}

impl Lifetime {
    pub fn new(max: Time) -> Self {
        Self {
            remaining: max,
            max,
        }
    }

    pub fn change(&mut self, delta: Time) {
        self.remaining = (self.remaining + delta).clamp(Time::ZERO, self.max);
    }

    pub fn ratio(&self) -> Time {
        if self.max == Time::ZERO {
            return Time::ZERO;
        }
        self.remaining / self.max
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

    pub fn alive(&self) -> bool {
        !matches!(self, TileState::Spawning(_) | TileState::Despawning(_))
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

/// All ghosts need a reason to exists, otherwise they perish into oblivion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExistentialReason {
    /// Another tile is moving.
    MoveFrom(vec2<ICoord>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TileKind {
    /// Not a real tile, but a placeholder to prevent stuff from happening.
    /// Used by animations and such.
    GhostBlock(ExistentialReason),
    Seed(Seed),
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
            TileKind::GhostBlock(_) => "Huh?",
            TileKind::Seed(seed) => match seed.kind {
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
            TileKind::GhostBlock(_) => "You are not supposed to see this",
            TileKind::Seed(seed) => match seed.kind {
                PlantKind::TypeA => "Grows from Dry Soil/ Soil/ Rich Soil",
                PlantKind::TypeB => "Grows from Soil/ Rich Soil",
                PlantKind::TypeC => "Grows only from Water",
                PlantKind::TypeD => "Grows only from Rich Soil",
            },
            TileKind::Leaf(leaf) => match leaf.kind {
                PlantKind::TypeA => "Sells for 3g",
                PlantKind::TypeB => "Sells for 10g",
                PlantKind::TypeC => "Sells for 50g",
                PlantKind::TypeD => "Sells for 30g",
            },
            TileKind::Light(_) => "Plants grow within range\nRequires Power to function",
            TileKind::Soil(state) => match state {
                SoilState::Dry => "Consumes adjacent Water and turns into soil",
                SoilState::Watered => {
                    "Consumes Poop nearby and turns into Rich Soil\nTurns into Dry Soil after plant growth"
                }
                SoilState::Rich => "Turns into Dry Soil after plant growth",
            },
            TileKind::Water(_) => "Spawns around leaves\nDisappears overtime",
            TileKind::Bug(_) => "Eats Plants and produces Poop\nSpawns in unlit areas",
            TileKind::Poop(_) => "Can be used to nourish the Soil\nDisappears overtime",
            TileKind::Power => "Provides power to tiles connected with Wires",
            TileKind::Wire(_) => {
                "Connection between Power and Light\nCan be destroyed by Bugs and Plants"
            }
            TileKind::Drainer => {
                "Collects Water within range to your inventory or to connected Sprinklers"
            }
            TileKind::Cutter(_) => "Automatically cuts fully grown adjacent Plants\nRequires Power",
            TileKind::Pipe(_) => "Connection between water collector and Sprinkler",
            TileKind::Sprinkler(_) => {
                "Ejects Water on adjacent tiles when connected to a Drainer via Pipes"
            }
            TileKind::Rock => "Blocks plants growth and bugs",
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

    pub fn action_progress(&self, config: &Config) -> Option<R32> {
        match self {
            TileKind::Leaf(leaf) => leaf
                .growth_timer
                .map(|t| Time::ONE - t)
                .filter(|&t| t > Time::ZERO),
            TileKind::Water(lifetime) | TileKind::Poop(lifetime) => {
                Some(lifetime.ratio()).filter(|&t| t < Time::ONE)
            }
            TileKind::Cutter(cutter) => {
                Some(Time::ONE - cutter.cut_timer.ratio()).filter(|&t| t > Time::ZERO)
            }
            TileKind::Bug(bug) => match &bug.state {
                BugState::Hungry { eating_timer, .. } => {
                    let t = eating_timer.ratio();
                    (t < Time::ONE).then_some(t)
                }
                BugState::Pooping(timer) => Some(timer.ratio()),
                _ => None,
            },
            TileKind::Seed(seed) => {
                let seed_energy = seed
                    .growth_energy
                    .values()
                    .copied()
                    .fold(R32::ZERO, R32::add);
                let config = config.plants.get(&seed.kind)?;
                Some(seed_energy.floor() / config.growth_capacity)
            }
            _ => None,
        }
    }

    /// `None` if tile does not need power.
    /// `Some(true)` if tile is powered.
    /// `Some(false)` if tile is disconnected from power.
    pub fn is_powered(&self) -> Option<bool> {
        match self {
            TileKind::Light(powered) => Some(*powered),
            TileKind::Wire(powered) => Some(*powered),
            TileKind::Cutter(cutter) => Some(cutter.powered),
            _ => None,
        }
    }

    pub fn action_range(&self, config: &Config) -> Option<ICoord> {
        match self {
            TileKind::Light(_) => Some(config.light_radius),
            TileKind::Drainer => Some(config.drainer_radius),
            TileKind::Cutter(_) => Some(1),
            TileKind::Sprinkler(_) => Some(1),
            _ => None,
        }
    }
}
