use super::*;

#[derive(Debug, Clone)]
pub enum DroneTarget {
    MoveTo(vec2<ICoord>),
    Collect(vec2<ICoord>),
    CutPlant(vec2<ICoord>),
    PlaceTile(vec2<ICoord>, TileKind),
    BuyTile(vec2<ICoord>, TileKind),
    KillBug(Id),
}

impl DroneTarget {
    pub fn action(&self) -> Option<DroneAction> {
        match self {
            DroneTarget::MoveTo(_) => None,
            DroneTarget::Collect(_) => Some(DroneAction::Collect),
            DroneTarget::CutPlant(_) => Some(DroneAction::CutPlant),
            DroneTarget::PlaceTile(..) | DroneTarget::BuyTile(..) => Some(DroneAction::PlaceTile),
            DroneTarget::KillBug(_) => Some(DroneAction::KillBug),
        }
    }

    pub fn name(&self) -> &'static str {
        self.action().map_or("", |action| action.name())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DroneAction {
    CutPlant,
    Collect,
    PlaceTile,
    KillBug,
}

impl DroneAction {
    pub fn name(&self) -> &'static str {
        match self {
            DroneAction::CutPlant => "Harvest",
            DroneAction::Collect => "Collect",
            DroneAction::PlaceTile => "Place",
            DroneAction::KillBug => "Kill",
        }
    }
}

#[derive(Debug)]
pub struct Drone {
    pub position: vec2<FCoord>,
    pub velocity: vec2<FCoord>,
    // TODO: queue
    pub target: Option<DroneTarget>,
    pub action_progress: R32,
}
