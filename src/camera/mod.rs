mod cursor_ray;
mod rts;

use crate::config::CameraConfig;

pub use cursor_ray::{CursorRay, CursorRayCalculator, CursorRayCameraTag};

use cursor_ray::CursorRayPlugin;

use bevy::{app::prelude::*, ecs::prelude::*, math::prelude::*};
use smooth_bevy_cameras::{
    LookTransformPlugin,
};


pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_plugin(LookTransformPlugin)
            .add_plugin(rts::Plugin::default())
            .add_plugin(CursorRayPlugin);
    }
}

pub fn spawn(
    commands: &mut Commands,
    config: &CameraConfig,
    eye: Vec3,
    target: Vec3,
) -> Entity {
    commands
        .spawn_bundle(rts::CameraBundle::new(
            Default::default(),
            Default::default(),
            eye,
            target,
        ))
        .insert(CursorRayCameraTag)
        .id()
}
