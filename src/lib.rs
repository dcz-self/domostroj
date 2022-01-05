#[cfg(test)]
#[macro_use]
extern crate assert_float_eq;
mod camera;
mod config;
mod cursor_tracker;
mod database;
mod edit_tools;
pub mod generate;
mod geometry;
mod immediate_mode;
mod picking;
mod plugin;
mod stress;

use camera::{create_camera_entity, CameraPlugin, CursorRay};
use cursor_tracker::{CursorPosition, CursorPositionPlugin};
use database::{open_voxel_database, save_map_to_db};
use edit_tools::EditToolsPlugin;
use immediate_mode::{ImmediateModePlugin, ImmediateModeTag};
use picking::{VoxelCursor, VoxelCursorRayImpact, VoxelPickingPlugin};
use plugin::EditorState;

pub use config::*;
pub use plugin::EditorPlugin;
