use crate::EditorState;

use super::{
    controller::{
        initialize_selection_controller, selection_control_system, selection_default_input_map,
    },
    pick_view::{initialize_pick_view, pick_view_system },
    view::{initialize_selection_view, selection_view_system},
};

use bevy::{app::prelude::*, ecs::prelude::*};

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_enter(EditorState::Editing)
                .with_system(initialize_selection_controller.system())
                .with_system(initialize_selection_view.system())
                .with_system(initialize_pick_view.system()),
        )
        .add_system_set(
            SystemSet::on_update(EditorState::Editing)
                .with_system(selection_control_system.system())
                .with_system(selection_default_input_map.system())
                .with_system(selection_view_system.system())
                .with_system(pick_view_system.system()),
        );
    }
}
