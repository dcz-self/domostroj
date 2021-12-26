use crate::EditorState;

use super::{
    drag_face::{
        drag_face_default_input_map, drag_face_tool_system, DragFaceEvents, DragFaceState,
    },
    edit_timeline::EditTimeline,
    selection::{SelectionEvents, SelectionPlugin},
    slicing::{ SliceHeight, setup_slicing_hint, update_slicing_hint, set_render_slice },
    slicing::user::{ switch_to_slicer, slicer_change_level },
    terraformer::{
        terraformer_default_input_map, terraformer_system, Terraformer, TerraformerEvents,
    },
    tool_switcher::tool_switcher_system,
    undo::undo_system,
    CurrentTool,
};

use feldspar::bb::core::Point3i;

use bevy::{app::prelude::*, ecs::prelude::*, prelude::AppBuilder};

/// Depends on the `VoxelPickingPlugin`.
pub struct EditToolsPlugin {
    chunk_shape: Point3i,
}

impl EditToolsPlugin {
    pub fn new(chunk_shape: Point3i) -> Self {
        Self { chunk_shape }
    }
}

impl Plugin for EditToolsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(SelectionPlugin)
            .insert_resource(EditTimeline::new(self.chunk_shape))
            .insert_resource(Terraformer::default())
            .insert_resource(CurrentTool::Slice)
            .insert_resource(SliceHeight(5))
            .add_event::<TerraformerEvents>()
            .add_event::<DragFaceEvents>()
            .add_event::<SelectionEvents>()
            .add_system_set(
                SystemSet::on_enter(EditorState::Editing)
                    .with_system(setup_slicing_hint.system())
            )
            .add_system_set(
                SystemSet::on_update(EditorState::Editing)
                    .with_system(undo_system.system())
                    .with_system(tool_switcher_system.system())
                    .with_system(terraformer_system.system())
                    .with_system(terraformer_default_input_map.system())
                    .with_system(drag_face_tool_system.system())
                    .with_system(drag_face_default_input_map.system())
                    .with_system(slicer_change_level.system())
                    .with_system(switch_to_slicer.system())
                    .with_system(update_slicing_hint.system())
                    .with_system(set_render_slice.system())
            );
    }
}
