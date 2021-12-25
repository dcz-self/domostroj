//! User input parts for slicing mode

use crate::VoxelCursorRayImpact;
use crate::edit_tools::CurrentTool;
use super::SliceHeight;

use bevy::{ecs::prelude::*, input::prelude::*};


/// Selects the voxel layer where the cubical Selection cursor was present.
pub fn switch_to_slicer(
    keyboard: Res<Input<KeyCode>>,
    cursor_voxel: Res<VoxelCursorRayImpact>,
    mut current_tool: ResMut<CurrentTool>,
    mut slice_level: ResMut<SliceHeight>,
) {
    if keyboard.just_pressed(KeyCode::Q) {
        println!("Switching to Slicing tool");
        *current_tool = CurrentTool::Slice;

        if let Some(voxel) = cursor_voxel.get_neighoring_voxel() {
            slice_level.0 = voxel.y();
            println!("level: {}", slice_level.0);
        }
    }
}


pub fn slicer_change_level(
    keyboard: Res<Input<KeyCode>>,
    current_tool: Res<CurrentTool>,
    mut slice_level: ResMut<SliceHeight>,
) {
    if let CurrentTool::Slice = *current_tool {
        if keyboard.just_pressed(KeyCode::M) {
            slice_level.0 += 1;
            println!("level: {}", slice_level.0);
        } else if keyboard.just_pressed(KeyCode::N) {
            slice_level.0 -= 1;
            println!("level: {}", slice_level.0);
        }
    }
}
