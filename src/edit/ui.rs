/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */

use baustein::indices::Index;
use baustein::prefab::PaletteVoxel;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
use rfd;
use std::cmp;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::thread;

use crate::CursorRay;
use crate::edit;
use crate::edit::CurrentTool;
use crate::edit::slice;


use baustein::traits::Space;


#[derive(Clone, Copy, PartialEq, Debug)]
struct State {
    tool: CurrentTool,
    slice_state: slice::State,
}

pub fn process(
    voxel_info: Res<Option<VoxelInfo>>,
    mut egui_ctx: ResMut<EguiContext>,
    mut tool: ResMut<CurrentTool>,
    mut slice_state: ResMut<slice::State>,
    events: Res<Mutex<Sender<edit::Event>>>,
) {
    let old_state = State { tool: *tool, slice_state: *slice_state };
    let events = events.lock().unwrap();
    let new_state = process_panel(&mut *egui_ctx, old_state, &*voxel_info, &events);
    if new_state != old_state {
        *tool = new_state.tool;
        *slice_state = new_state.slice_state;
    }
}

pub struct VoxelInfo {
    index: Index,
    contents: slice::VoxelType,
}

pub fn update_voxel_info(
    space: Res<edit::World>,
    cursor_ray: Res<CursorRay>,
    slice_state: ResMut<slice::State>,
    tool: ResMut<CurrentTool>,
    mut voxel_info: ResMut<Option<VoxelInfo>>,
) {
    if let CurrentTool::Slice = *tool {
        *voxel_info = slice::find_cursor_voxel(&*cursor_ray, &slice_state.slice_height)
            .map(|index| VoxelInfo {
                index,
                contents: space.0.get(index),
            });
    }
}

fn process_panel(
    egui_ctx: &mut EguiContext,
    mut ui_state: State,
    voxel_info: &Option<VoxelInfo>,
    mut events: &Sender<edit::Event>,
) -> State {
    egui::SidePanel::left("side_panel")
        .show(egui_ctx.ctx(), |ui| {
            ui.heading("Tool");

            let tool = &mut ui_state.tool;
            ui.radio_value(tool, CurrentTool::DragFace, "Drag");
            ui.radio_value(tool, CurrentTool::Terraform, "Terraform");
            ui.radio_value(tool, CurrentTool::Slice, "Slice");

            if let CurrentTool::Slice = tool {
                let slice_level = &mut ui_state.slice_state.slice_height.0;
                ui.add(
                    // This slider could benefit from buttons on either end.
                    // And some 0-level indicator?
                    egui::Slider::new(slice_level, cmp::min(-32, *slice_level)..=cmp::max(32, *slice_level))
                        .vertical()
                        .text("level")
                );

                ui.horizontal(|ui| {
                    let voxel = &mut ui_state.slice_state.voxel_type;
                    // Those *really* need pictures.
                    ui.radio_value(voxel, PaletteVoxel(0), "Empty");
                    ui.radio_value(voxel, PaletteVoxel(1), "Grass");
                    ui.radio_value(voxel, PaletteVoxel(2), "Ground");
                    ui.radio_value(voxel, PaletteVoxel(3), "Ice");
                    ui.radio_value(voxel, PaletteVoxel(4), "?");
                });
            }

            if let Some(VoxelInfo { index, contents }) = voxel_info {
                ui.heading("Voxel");
    
                ui.label(format!("{:?}", index));
                ui.label(format!("{:?}", contents));
            }
            
            ui.heading("Scene");
            if ui.button("Load…").clicked() {
                let sender = events.clone();
                thread::spawn(move || {
                    let path = rfd::FileDialog::new()
                        .add_filter("Domostroj scene", &["domo"])
                        .set_directory(".")
                        .pick_file();
                    if let Some(path) = path {
                        sender
                            .send(edit::Event::LoadFile(path))
                            .unwrap_or_else(|e| eprintln!("Can't load: {:?}", e));
                    }
                });
            }
            if ui.button("Save…").clicked() {
                let sender = events.clone();
                thread::spawn(move || {
                    let path = rfd::FileDialog::new()
                        .add_filter("Domostroj scene", &["domo"])
                        .set_directory(".")
                        .save_file();
                    if let Some(path) = path {
                        sender
                            .send(edit::Event::SaveFile(path))
                            .unwrap_or_else(|e| eprintln!("Can't save: {:?}", e));
                    }
                });
            }
        });
    ui_state
}
