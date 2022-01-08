//! User input parts for slicing mode

use crate::VoxelCursorRayImpact;
use crate::edit::CurrentTool;
use super::State;

use bevy::{ecs::prelude::*, input::prelude::*};


/// Selects the voxel layer where the cubical Selection cursor was present.
pub fn switch_to_slicer(
    keyboard: Res<Input<KeyCode>>,
    cursor_voxel: Res<VoxelCursorRayImpact>,
    mut current_tool: ResMut<CurrentTool>,
    mut state: ResMut<State>,
) {
    if keyboard.just_pressed(KeyCode::Q) {
        println!("Switching to Slicing tool");
        *current_tool = CurrentTool::Slice;

        if let Some(voxel) = cursor_voxel.get_neighoring_voxel() {
            state.slice_height.0 = voxel.y();
            println!("level: {}", state.slice_height.0);
        }
    }
}


pub fn change_level(
    keyboard: Res<Input<KeyCode>>,
    current_tool: Res<CurrentTool>,
    mut state: ResMut<State>,
) {
    if let CurrentTool::Slice = *current_tool {
        if keyboard.just_pressed(KeyCode::M) {
            state.slice_height.0 += 1;
            println!("level: {}", state.slice_height.0);
        } else if keyboard.just_pressed(KeyCode::N) {
            state.slice_height.0 -= 1;
            println!("level: {}", state.slice_height.0);
        }
    }
}
