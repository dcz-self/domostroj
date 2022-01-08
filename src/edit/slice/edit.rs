use crate::{CursorRay, VoxelCursor};
use crate::edit::{ CurrentTool, PaletteVoxel };

use bevy::{ecs::prelude::*, input::prelude::*, prelude::*};

use feldspar::bb::{core::prelude::*, storage::prelude::Sd8};

use super::{ State, VoxelType, World, find_cursor_voxel };


pub enum Events {
    ChangeVoxelType(u8),
    MakeSolid,
    RemoveSolid,
}

pub fn input_map(
    mut events: EventWriter<Events>,
    keyboard: Res<Input<KeyCode>>,
) {
    // Adjust the voxel type to create.
    if keyboard.just_pressed(KeyCode::Key1) {
        events.send(Events::ChangeVoxelType(1));
    } else if keyboard.just_pressed(KeyCode::Key2) {
        events.send(Events::ChangeVoxelType(2));
    } else if keyboard.just_pressed(KeyCode::Key3) {
        events.send(Events::ChangeVoxelType(3));
    } else if keyboard.just_pressed(KeyCode::Key4) {
        events.send(Events::ChangeVoxelType(4));
    }

    if keyboard.pressed(KeyCode::Z) {
        events.send(Events::MakeSolid);
    } else if keyboard.pressed(KeyCode::X) {
        events.send(Events::RemoveSolid);
    }
}

pub fn update_state(
    cursor_ray: Res<CursorRay>,
    current_tool: Res<CurrentTool>,
    mut space: ResMut<crate::edit::World>,
    mut state: ResMut<State>,
    mut events: EventReader<Events>,
) {
    if let CurrentTool::Slice = *current_tool {
    } else {
        return;
    }

    for event in events.iter() {
        match event {
            Events::MakeSolid => {
                if let Some(index) = find_cursor_voxel(&*cursor_ray, &state.slice_height) {
                    space.0.set(index, state.voxel_type)
                        .unwrap_or_else(|e| eprintln!("{:?} {:?}", e, index));
                }
            },
            Events::RemoveSolid => {
                if let Some(index) = find_cursor_voxel(&*cursor_ray, &state.slice_height) {
                    space.0.set(index, VoxelType::EMPTY)
                        .unwrap_or_else(|e| eprintln!("{:?} {:?}", e, index));
                }
            },
            Events::ChangeVoxelType(voxel_type) => {
                state.voxel_type = PaletteVoxel(*voxel_type);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Operation {
    MakeSolid,
    RemoveSolid,
}
