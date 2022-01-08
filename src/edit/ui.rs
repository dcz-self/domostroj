use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};

pub fn ui(
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<UiState>,
    assets: Res<AssetServer>,
) {
    let mut load = false;
    let mut remove = false;
    let mut invert = false;

    egui::SidePanel::left("side_panel")
        //.default_width(200.0)
        .show(egui_ctx.ctx(), |ui| {
            ui.heading("Tool");

            let mut tool = current_tool.clone();
            ui.radio_value(&mut tool, CurrentTool::DragFace, "Drag");
            ui.radio_value(&mut tool, CurrentTool::Terraform, "Terraform");
            ui.radio_value(&mut tool, CurrentTool::Slice, "Slice");

            if let tool == CurrentTool::Slice {
                let mut slice_level = slice_level.clone();
                ui.add(
                    egui::Slider::new(&mut slice_level, -32..=32)
                        .vertical()
                        .text("level")
                );

                ui.horizontal(|ui| {
                    let mut voxel = current_voxel.clone();
                    ui.radio_value(&mut voxel, PaletteVoxel(0), "Empty");
                    ui.radio_value(&mut voxel, PaletteVoxel(0), "Grass");
                    ui.radio_value(&mut voxel, PaletteVoxel(0), "Ground");
                    ui.radio_value(&mut voxel, PaletteVoxel(0), "Ice");
                    ui.radio_value(&mut voxel, PaletteVoxel(0), "?");
                });
            }
        });
}
