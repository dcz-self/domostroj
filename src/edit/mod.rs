/*! Edit mode:
 * world, rendering, UI. */

mod ui;
mod slice;

use crate::EditorState;
 
use baustein::indices::to_i32_arr;
use baustein::prefab::PaletteVoxel;
use baustein::re::{ ConstPow2Shape, ConstShape };
use baustein::render::{ mesh_from_quads, MeshMaterial };
use baustein::traits::Space;
use baustein::world::FlatPaddedGridCuboid;
use bevy::app::AppBuilder;
use bevy::asset::Assets;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::schedule::SystemSet;
use bevy::ecs::system::{ Commands, Query, Res, ResMut };
use bevy::render::mesh::Mesh;
use bevy::transform::components::Transform;
use block_mesh;
use block_mesh::{ greedy_quads, GreedyQuadsBuffer, MergeVoxel, UnorientedQuad, RIGHT_HANDED_Y_UP_CONFIG };
use feldspar::prelude::create_voxel_mesh_bundle;
use std::error::Error;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;


use baustein::traits::Extent;
use bevy::prelude::IntoSystem;


/// A wrapper over a mundane chunk, for the purpose of becoming the Bevy resource.
#[derive(Clone)]
pub struct World(FlatPaddedGridCuboid<PaletteVoxel, ConstPow2Shape<5, 5, 5>>);

/// Create a default World with a grassy, diggable floor below level 0.
pub fn floor() -> World {
    let extent = FlatPaddedGridCuboid::<(), ConstPow2Shape<5, 5, 5>>::new([0, -8, 0].into());
    let world = extent.map_index(|i, _| {
        if i.y() < 0 {
            PaletteVoxel(1) // hopefully grass
        } else {
            PaletteVoxel::EMPTY
        }
    });
    World(world.into())
}

pub enum Event {
    LoadFile(PathBuf),
    SaveFile(PathBuf),
}

pub fn handle_events(
    space: Res<World>,
    mut events: Res<Mutex<Receiver<Event>>>,
) {
    let events = events.try_lock();
    if let Ok(events) = events {
        for event in events.try_iter() {
            use Event::*;
            match event {
                LoadFile(path) => {println!("Load")},
                SaveFile(path) => {
                    let space = (*space).clone();
                    thread::spawn(move ||
                        save(space, path)
                            .unwrap_or_else(|e| eprintln!("Failed to save: {:?}", e))
                    );
                }
            }
        }
    }
}

use bincode;
use std::fs;
fn save(world: World, path: PathBuf) -> Result<(), Box<dyn Error>>{
    let f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)?;
    let mut f = BufWriter::new(f);
    bincode::serialize_into(&mut f, &world.0)?;
    Ok(())
}

// Older version needed for block_mesh
type BlockMeshShape = block_mesh::ndshape::ConstPow2Shape3u32::<5, 5, 5>;

/// Marks which meshes should despawn
pub struct EditMesh;

pub fn update_meshes(
    mut commands: Commands,
    mesh_material: Res<MeshMaterial>,
    mesh_cutoff: Res<slice::MeshCutoff>,
    space: Res<World>,
    mut meshes: ResMut<Assets<Mesh>>,
    edit_meshes: Query<Entity, With<EditMesh>>,
) {
    // Get rid of all meshes
    for cm in edit_meshes.iter() {
        commands.entity(cm).despawn()
    }
    // And create the occupied ones again.
    // Wasteful, I know. I'm testing!

    // The creation of this space can be optimized:
    // create an extent that doesn't go above the offset.
    // This is annoying, so it's skipped.
    let mesh_cutoff = mesh_cutoff.0 as i32;
    let space = space.0.map_index(|i, v| {
        if i.y() < mesh_cutoff { v }
        else { Default::default() }
    });
    type Shape = ConstPow2Shape<5, 5, 5>;
    let space = FlatPaddedGridCuboid::<_, Shape>::new_from_space(&space, space.get_offset());
    
    let quads = generate_greedy_buffer_fast(&space);
    let material_lookup = |quad: &UnorientedQuad| {
        let i = space.get(to_i32_arr(quad.minimum).into()).0;
        let mut material = [0; 4];
        material[i as usize] = 1;
        [material, material, material, material]
    };
    let mesh = mesh_from_quads(quads, &space, material_lookup);
    if let Some((mesh, materials)) = mesh {
        commands
            .spawn_bundle(
                create_voxel_mesh_bundle(
                    mesh,
                    materials,
                    mesh_material.0.clone(),
                    &mut meshes,
                )
            )
            .insert(Transform::from_translation(space.get_offset().into()))
            .insert(EditMesh)
            ;
    }
}


fn generate_greedy_buffer_fast<V, Shape>(
    view: &FlatPaddedGridCuboid<V, Shape>,
) -> GreedyQuadsBuffer
    where
    V: MergeVoxel + Copy + Default,
    Shape: ConstShape,
{
    let samples = view.get_samples();
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = GreedyQuadsBuffer::new(samples.len());

    greedy_quads(
        samples,
        &BlockMeshShape {},
        [0, 0, 0],
        [
            <Shape as ConstShape>::ARRAY[0] as u32 - 1,
            <Shape as ConstShape>::ARRAY[1] as u32 - 1,
            <Shape as ConstShape>::ARRAY[2] as u32 - 1,
        ],
        &faces,
        &mut buffer,
    );
    buffer
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum CurrentTool {
    DragFace,//(DragFaceState),
    Terraform,
    Slice,
}


/// Depends on the `VoxelPickingPlugin`.
pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut AppBuilder) {
        let (ui_sender, ui_receiver) = mpsc::channel::<Event>();
        app
            .insert_resource(floor())
            .insert_resource(CurrentTool::Slice)
            .insert_resource(slice::State::default())
            .insert_resource(slice::MeshCutoff::default())
            .insert_resource(Option::<ui::VoxelInfo>::None)
            .insert_resource(Mutex::new(ui_sender))
            .insert_resource(Mutex::new(ui_receiver))
            .add_event::<slice::edit::Events>()
//            .add_event::<DragFaceEvents>()
//            .add_event::<SelectionEvents>()
            .add_system_set(
                SystemSet::on_enter(EditorState::Editing)
                    .with_system(slice::setup_hint.system())
            )
            .add_system_set(
                SystemSet::on_update(EditorState::Editing)
//                    .with_system(undo_system.system())
  //                  .with_system(tool_switcher_system.system())
    //                .with_system(terraformer_system.system())
      //              .with_system(terraformer_default_input_map.system())
        //            .with_system(drag_face_tool_system.system())
          //          .with_system(drag_face_default_input_map.system())
                    .with_system(slice::user::change_level.system())
                    .with_system(slice::user::switch_to_slicer.system())
                    .with_system(slice::update_hint.system())
                    .with_system(slice::set_render_slice.system())
                    .with_system(slice::edit::update_state.system())
                    .with_system(slice::edit::input_map.system())
                    .with_system(slice::show_mesh_count.system())
                    .with_system(ui::process.system())
                    .with_system(ui::update_voxel_info.system())
                    .with_system(handle_events.system())
            );
    }
}

#[cfg(test)]
mod test {
    use super::*;

}
