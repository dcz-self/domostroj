use crate::{CursorRay, VoxelCursor};
use crate::edit_tools::{CurrentTool, SnapshottingVoxelEditor};

use bevy::{ecs::prelude::*, input::prelude::*, prelude::*};

use feldspar::bb::{core::prelude::*, storage::prelude::Sd8};
use feldspar::prelude::VoxelType;

use super::{ State, find_cursor_voxel };


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
    mut state: ResMut<State>,
    mut voxel_editor: SnapshottingVoxelEditor,
    mut events: EventReader<Events>,
) {
    if let CurrentTool::Slice = *current_tool {
    } else {
        return;
    }

    for event in events.iter() {
        match event {
            Events::MakeSolid => {
                if let Some(voxel) = find_cursor_voxel(&*cursor_ray, &state.slice_height) {
                    edit_voxel(
                        Operation::MakeSolid,
                        voxel,
                        state.voxel_type,
                        &mut voxel_editor,
                    );
                }
            },
            Events::RemoveSolid => {
                if let Some(voxel) = find_cursor_voxel(&*cursor_ray, &state.slice_height) {
                    edit_voxel(
                        Operation::RemoveSolid,
                        voxel,
                        VoxelType::EMPTY,
                        &mut voxel_editor,
                    );
                }
            },
            Events::ChangeVoxelType(voxel_type) => {
                state.voxel_type = VoxelType(*voxel_type);
            }
        }
    }
}

fn edit_voxel(
    operation: Operation,
    center: Point3i,
    voxel_type: VoxelType,
    voxel_editor: &mut SnapshottingVoxelEditor,
) {
    let sign = match operation {
        Operation::MakeSolid => -1,
        Operation::RemoveSolid => 1,
    };
    voxel_editor.edit_extent_and_touch_neighbors(
        unit_extent(center),
        |p: Point3i, (v_type, _v_dist): (&mut VoxelType, &mut Sd8)| {
            *v_type = voxel_type;
            // maybe TODO: SDF calculation.
        },
    );
}

fn unit_extent(center: Point3i) -> Extent3i {
    Extent3i::from_min_and_shape(center, PointN([1; 3]))
}

#[derive(Clone, Copy)]
enum Operation {
    MakeSolid,
    RemoveSolid,
}
