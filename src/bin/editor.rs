use domostroj::{generate, Config, EditorPlugin};

use bevy::app::prelude::*;

fn main() -> Result<(), ron::Error> {
    env_logger::Builder::from_default_env()
        // Filter out some noisy crates
        .filter(Some("gfx_backend_metal"), log::LevelFilter::Error)
        .filter(Some("naga"), log::LevelFilter::Error)
        .init();

    let config = Config::read_file("config.ron")?;

    App::build()
        .add_plugin(EditorPlugin::new(config))
        .add_plugin(generate::CameraPlugin)
        .add_plugin(generate::render::Plugin)
        .run();

    Ok(())
}
