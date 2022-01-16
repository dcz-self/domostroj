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
use crate::generate;
use crate::generate::scene;


use baustein::traits::Space;


#[derive(Clone, Copy, PartialEq, Debug)]
struct State {
    //slice_state: slice::State,
}

/// Draws UI and applies state changes.
pub fn process(
    window_id: Res<generate::Window>,
    mut egui_ctx: ResMut<EguiContext>,
    //mut slice_state: ResMut<slice::State>,
    events: Res<Mutex<Sender<generate::Event>>>,
) {
    let old_state = State { };//slice_state: *slice_state };
    let events = events.lock().unwrap();
    let ctx = match egui_ctx.try_ctx_for_window(window_id.0) {
        Some(k) => k,
        None => {return;},
    };
    let new_state = process_panel(ctx, old_state, &events);
    if new_state != old_state {
        //*slice_state = new_state.slice_state;
    }
}

/// Draws panel and sends messages.
fn process_panel(
    egui_ctx: &egui::CtxRef,
    mut ui_state: State,
    mut events: &Sender<generate::Event>,
) -> State {
    egui::SidePanel::left("side_panel")
        .show(egui_ctx, |ui| {
            ui.heading("Generator");
            if ui.button("Update stamps").clicked() {
                events.send(generate::Event::StepOne).unwrap();
            }
            if ui.button("1 Step").clicked() {
                events.send(generate::Event::StepOne).unwrap();
            }

            ui.heading("Info");
            /*
            {
                let slice_level = &mut ui_state.slice_state.slice_height.0;
                ui.add(
                    // This slider could benefit from buttons on either end.
                    // And some 0-level indicator?
                    egui::Slider::new(slice_level, cmp::min(-32, *slice_level)..=cmp::max(32, *slice_level))
                        .vertical()
                        .text("level")
                );
            }*/
        });
    ui_state
}
