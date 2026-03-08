mod font;

pub use self::font::*;

use crate::{model::*, prelude::Color};

use std::path::PathBuf;

use geng::prelude::*;
use geng_utils::gif::GifFrame;

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
}

#[derive(geng::asset::Load)]
pub struct Sprites {
    pub drone: PixelTexture,
    pub tile_select: PixelTexture,
    pub tile: PixelTexture,
    pub tiles: SpritesTiles,

    pub ui_window: PixelTexture,
    pub ui_window_shop: PixelTexture,
}

#[derive(geng::asset::Load)]
pub struct SpritesTiles {
    pub plant: PixelTexture,
    pub light: PixelTexture,
    pub seed: PixelTexture,
    pub soil_dry: PixelTexture,
    pub soil: PixelTexture,
    // pub soil_rich: PixelTexture,
}

impl SpritesTiles {
    pub fn get_texture(&self, tile: &Tile) -> &PixelTexture {
        match tile {
            Tile::Leaf(_) => &self.plant,
            Tile::Light => &self.light,
            Tile::Seed(_) => &self.seed,
            Tile::Soil(state) => match state {
                SoilState::Dry => &self.soil_dry,
                SoilState::Watered => &self.soil,
            },
        }
    }
}

#[derive(geng::asset::Load)]
pub struct Sounds {
    pub ui_click: Rc<geng::Sound>,
    pub ui_hover: Rc<geng::Sound>,
}

#[derive(geng::asset::Load)]
pub struct Shaders {
    pub texture: Rc<ugli::Program>,
    pub ellipse: Rc<ugli::Program>,
    pub solid: Rc<ugli::Program>,
}

#[derive(geng::asset::Load)]
pub struct Fonts {
    pub default: Rc<Font>,
}

#[derive(geng::asset::Load)]
pub struct Assets {
    pub palette: Palette,
    pub sprites: Sprites,
    pub sounds: Sounds,
    pub shaders: Shaders,
    pub fonts: Fonts,
}

impl Assets {
    pub async fn load(manager: &geng::asset::Manager) -> anyhow::Result<Self> {
        geng::asset::Load::load(manager, &run_dir().join("assets"), &()).await
    }
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
