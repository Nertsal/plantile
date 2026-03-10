mod font;

pub use self::font::*;

use crate::{model::*, prelude::Color};

use std::path::PathBuf;

use geng::prelude::*;
use geng_utils::gif::GifFrame;

#[derive(geng::asset::Load, Serialize, Deserialize, Debug, Clone)]
#[load(serde = "ron")]
pub struct Config {
    pub drone_acceleration: R32,
    pub drone_deceleration: R32,
    pub drone_max_speed: R32,
    pub drone_reach: R32,

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

    pub light_radius: ICoord,
    pub drainer_radius: ICoord,
    pub cutter_radius: ICoord,
    pub cutter_cooldown: Time,

    pub plants: HashMap<PlantKind, ConfigPlant>,
    pub shop: Vec<ConfigShopItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPlant {
    pub growth_time: Time,
    pub growth_time_dark: Time,
    pub max_size: usize,
    pub price: Money,
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

#[derive(geng::asset::Load)]
pub struct Assets {
    pub palette: Palette,
    pub sprites: Sprites,
    pub sounds: Sounds,
    pub shaders: Shaders,
    pub fonts: Fonts,
    pub config: Config,
}

impl Assets {
    pub async fn load(manager: &geng::asset::Manager) -> anyhow::Result<Self> {
        geng::asset::Load::load(manager, &run_dir().join("assets"), &()).await
    }
}

#[derive(geng::asset::Load)]
pub struct LoadingAssets {
    #[load(path = "sprites/title.png", options(filter = "ugli::Filter::Nearest"))]
    pub title: ugli::Texture,
    #[load(path = "fonts/default.ttf")]
    pub font: Font,
    #[load(load_with = "load_gif(&manager, &base_path.join(\"sprites/loading_background.gif\"))")]
    pub background: Vec<GifFrame>,
}

fn load_gif(
    manager: &geng::asset::Manager,
    path: &std::path::Path,
) -> geng::asset::Future<Vec<GifFrame>> {
    let manager = manager.clone();
    let path = path.to_owned();
    async move {
        geng_utils::gif::load_gif(
            &manager,
            &path,
            geng_utils::gif::GifOptions {
                frame: geng::asset::TextureOptions {
                    filter: ugli::Filter::Nearest,
                    ..Default::default()
                },
            },
        )
        .await
    }
    .boxed_local()
}

#[derive(Debug, Clone, geng::asset::Load, Serialize, Deserialize)]
#[load(serde = "toml")]
pub struct Palette {
    pub background: Color,
    pub text: Color,
    pub gold: Color,
    pub progress_background: Color,
    pub progress: Color,
    pub connection_plant: Color,
    pub connection_power: Color,
}

#[derive(geng::asset::Load)]
pub struct Sprites {
    pub drone: PixelTexture,
    pub tile_select: PixelTexture,
    pub tile: PixelTexture,
    pub tiles: SpritesTiles,
    pub tile_connector_horizontal: PixelTexture,
    pub tile_connector_vertical: PixelTexture,

    pub ui_window: PixelTexture,
    // pub ui_window_shop: PixelTexture,
}

#[derive(geng::asset::Load)]
pub struct SpritesTiles {
    pub plant_a: PixelTexture,
    pub plant_b: PixelTexture,
    pub plant_c: PixelTexture,
    pub plant_d: PixelTexture,
    pub light: PixelTexture,
    pub seed_a: PixelTexture,
    pub seed_b: PixelTexture,
    pub seed_c: PixelTexture,
    pub seed_d: PixelTexture,
    pub soil_dry: PixelTexture,
    pub soil: PixelTexture,
    pub soil_rich: PixelTexture,
    pub water: PixelTexture,
    pub bug: PixelTexture,
    pub poop: PixelTexture,
    pub power: PixelTexture,
    pub wire: PixelTexture,
    pub drain: PixelTexture,
    pub cutter: PixelTexture,
    pub pipe: PixelTexture,
    pub sprinkler: PixelTexture,
    pub rock: PixelTexture,
}

impl SpritesTiles {
    pub fn get_texture(&self, tile: &TileKind) -> &PixelTexture {
        match tile {
            TileKind::Leaf(leaf) => match leaf.kind {
                PlantKind::TypeA => &self.plant_a,
                PlantKind::TypeB => &self.plant_b,
                PlantKind::TypeC => &self.plant_c,
                PlantKind::TypeD => &self.plant_d,
            },
            TileKind::Seed(kind) => match kind {
                PlantKind::TypeA => &self.seed_a,
                PlantKind::TypeB => &self.seed_b,
                PlantKind::TypeC => &self.seed_c,
                PlantKind::TypeD => &self.seed_d,
            },
            TileKind::Light(_) => &self.light,
            TileKind::Soil(state) => match state {
                SoilState::Dry => &self.soil_dry,
                SoilState::Watered => &self.soil,
                SoilState::Rich => &self.soil_rich,
            },
            TileKind::Water(_) => &self.water,
            TileKind::Bug(_) => &self.bug,
            TileKind::Poop(_) => &self.poop,
            TileKind::Power => &self.power,
            TileKind::Wire(_) => &self.wire,
            TileKind::Drainer => &self.drain,
            TileKind::Cutter(_) => &self.cutter,
            TileKind::Pipe(_) => &self.pipe,
            TileKind::Sprinkler(_) => &self.sprinkler,
            TileKind::Rock => &self.rock,
        }
    }
}

#[derive(geng::asset::Load)]
pub struct Sounds {
    pub ui_click: Rc<geng::Sound>,
    pub ui_hover: Rc<geng::Sound>,

    pub bug_eat: Rc<geng::Sound>,
    pub bug_poop: Rc<geng::Sound>,
    pub drone_confirm: Rc<geng::Sound>,
    pub grow: Rc<geng::Sound>,
    pub rock: Rc<geng::Sound>,
}

#[derive(geng::asset::Load)]
pub struct Shaders {
    pub texture: Rc<ugli::Program>,
    pub ellipse: Rc<ugli::Program>,
    pub solid: Rc<ugli::Program>,
}

#[derive(geng::asset::Load)]
pub struct Fonts {
    // pub default: Rc<Font>,
    pub aseprite: Rc<Font>,
}

#[derive(Clone)]
pub struct PixelTexture {
    pub path: PathBuf,
    pub texture: Rc<ugli::Texture>,
}

impl Deref for PixelTexture {
    type Target = ugli::Texture;

    fn deref(&self) -> &Self::Target {
        &self.texture
    }
}

impl Debug for PixelTexture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PixelTexture")
            .field("path", &self.path)
            .field("texture", &"<texture data>")
            .finish()
    }
}

impl geng::asset::Load for PixelTexture {
    type Options = <ugli::Texture as geng::asset::Load>::Options;

    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        options: &Self::Options,
    ) -> geng::asset::Future<Self> {
        let path = path.to_owned();
        let texture = ugli::Texture::load(manager, &path, options);
        async move {
            let mut texture = texture.await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Self {
                path,
                texture: Rc::new(texture),
            })
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}
