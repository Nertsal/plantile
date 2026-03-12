use super::*;

#[derive(Debug, Clone)]
pub enum DroneTarget {
    MoveTo(vec2<ICoord>),
    Interact(vec2<ICoord>, DroneAction),
    PlaceTile(vec2<ICoord>, TileKind),
    BuyTile(vec2<ICoord>, TileKind),
    KillBug(Id),
}

impl DroneTarget {
    pub fn name(&self) -> &'static str {
        match self {
            DroneTarget::MoveTo(_) => "",
            DroneTarget::Interact(_, action) => action.name(),
            DroneTarget::PlaceTile(..) | DroneTarget::BuyTile(..) => "Place",
            DroneTarget::KillBug(_) => "Kill",
        }
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
    pub target: DroneTarget,
    pub action_progress: R32,
}
