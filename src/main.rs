mod assets;
mod context;
mod game;
mod menu;
mod model;
mod prelude;
mod render;
mod task;
mod ui;
mod util;

use anyhow::Result;
use geng::prelude::*;

const OPTIONS_STORAGE: &str = "options";

#[derive(clap::Parser)]
struct Opts {
    #[clap(flatten)]
    geng: geng::CliArgs,
    #[clap(long)]
    log: Option<String>,
}

fn main() {
    log::info!("Hello, gamers!");

    let opts: Opts = clap::Parser::parse();

    let mut builder = logger::builder();
    builder
        .filter_level(
            if let Some(level) = opts.log.as_deref().or(option_env!("LOG")) {
                match level {
                    "trace" => log::LevelFilter::Trace,
                    "debug" => log::LevelFilter::Debug,
                    "info" => log::LevelFilter::Info,
                    "warn" => log::LevelFilter::Warn,
                    "error" => log::LevelFilter::Error,
                    "off" => log::LevelFilter::Off,
                    _ => panic!("invalid log level string"),
                }
            } else if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            },
        )
        .filter_module("calloop", log::LevelFilter::Debug);
    logger::init_with(builder).expect("failed to init logger");
    geng::setup_panic_handler();

    let mut options = geng::ContextOptions::default();
    options.with_cli(&opts.geng);
    options.window.title = "Nertplate".into();

    Geng::run_with(&options, |geng| async move {
        let main = geng_main(geng, opts);

        #[cfg(not(target_arch = "wasm32"))]
        let main = async_compat::Compat::new(main);

        match main.await {
            Ok(()) => {}
            Err(err) => {
                log::error!("Application failed: {}", err);
                log::debug!("Full error: {:?}", err);
                std::process::exit(1);
            }
        }
    });

    log::info!("Please come back...");
}

async fn geng_main(geng: Geng, _opts: Opts) -> Result<()> {
    log::debug!("Initializing the loading screen...");
    let loading_assets: Rc<assets::LoadingAssets> =
        geng::asset::Load::load(geng.asset_manager(), &run_dir().join("assets"), &())
            .await
            .context("when loading assets")?;

    let load_everything = load_everything(geng.clone());
    let loading_screen = menu::LoadingScreen::new(&geng, loading_assets, load_everything).run();

    let context = loading_screen
        .await
        .ok_or_else(|| anyhow::Error::msg("loading screen failed"))??;

    log::debug!("Loading complete!");

    let state = game::GameState::new(context);
    geng.run_state(state).await;

    Ok(())
}

async fn load_everything(geng: Geng) -> Result<context::Context> {
    let manager = geng.asset_manager();

    let assets = assets::Assets::load(manager).await?;
    let assets = Rc::new(assets);

    let context = context::Context::new(&geng, &assets)
        .await
        .expect("failed to initialize context");

    Ok(context)
}
