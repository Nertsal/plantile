use super::*;

use linear_map::LinearMap;

#[derive(geng::asset::Load, Serialize, Deserialize, Debug, Clone)]
#[load(serde = "ron")]
pub struct Config {
    pub drone_acceleration: R32,
    pub drone_deceleration: R32,
    pub drone_max_speed: R32,
    pub drone_reach: R32,

    pub bug_population: usize,

    pub seed_grow_only_up: bool,

    pub action_duration: HashMap<DroneAction, Time>,

    pub rock_frequency: R32,
    pub water_frequency: R32,
    pub water_lifetime: Time,
    pub poop_lifetime: Time,

    pub bug_frequency: R32,
    pub bug_hunger: usize,
    pub bug_eat_time: Time,
    pub bug_poop_time: Time,
    pub bug_chill_time: Time,
    pub bug_move_time: Time,
    pub bug_vision_radius: ICoord,

    pub light_radius: ICoord,
    pub drainer_radius: ICoord,
    pub cutter_cut_time: Time,

    pub plants: HashMap<PlantKind, ConfigPlant>,
    pub shop: Vec<ConfigShopItem>,

    pub animations: ConfigAnimations,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigAnimations {
    pub tile_spawn: Time,
    pub tile_despawn: Time,
    pub bug_move: Time,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPlant {
    pub growth_time: Time,
    pub growth_time_dark: Time,
    pub max_size: usize,
    pub price: Money,
    // How many leaves grow when fully charged from soil before needing recharge.
    pub growth_capacity: R32,
    pub soils: LinearMap<TileKind, ConfigPlantSoil>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPlantSoil {
    // Growth speed multiplier.
    pub growth_speed: Time,
    // How many leaves grow from a single soil.
    pub capacity: R32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigShopItem {
    pub price: Money,
    pub unlocked_at: Money,
    pub tile: TileKind,
}

impl Config {
    pub fn get_cost(&self, tile: &TileKind) -> Money {
        self.shop
            .iter()
            .find(|item| item.tile == *tile)
            .map(|item| item.price)
            .unwrap_or(0)
    }
}
