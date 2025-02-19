#[cfg(test)]
#[macro_use]
extern crate assert_float_eq;
#[macro_use]
extern crate rental;

mod analyze;
mod camera;
mod config;
mod cursor_tracker;
mod database;
mod edit;
pub mod generate;
mod geometry;
mod immediate_mode;
mod picking;
mod plugin;
mod stress;
mod util;

use camera::CursorRay;
use cursor_tracker::{CursorPosition, CursorPositionPlugin};
use database::{open_voxel_database, save_map_to_db};
use immediate_mode::{ImmediateModePlugin};
use picking::{VoxelCursor, VoxelCursorRayImpact, VoxelPickingPlugin};
use plugin::EditorState;

pub use config::*;
pub use plugin::EditorPlugin;
