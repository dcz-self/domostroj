/*! Edit mode:
 * world, rendering, UI. */

mod ui;
mod slice;

use crate::camera;
use crate::config::{ CameraConfig, Config };
use crate::EditorState;
 
use baustein::indices::{to_i32_arr, VoxelUnits};
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
use bevy::math::Vec3;
use bevy::render::mesh::Mesh;
use bevy::transform::components::Transform;
use bincode;
use block_mesh;
use block_mesh::{ greedy_quads, GreedyQuadsBuffer, MergeVoxel, UnorientedQuad, RIGHT_HANDED_Y_UP_CONFIG };
use feldspar::prelude::create_voxel_mesh_bundle;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;


use baustein::traits::Cuboid as Extent;
use bevy::prelude::IntoSystem;
use bevy::prelude::ParallelSystemDescriptorCoercion;

pub type Shape = ConstPow2Shape<5, 5, 5>;

/// A wrapper over a mundane chunk, for the purpose of becoming the Bevy resource.
#[derive(Clone)]
pub struct World(pub FlatPaddedGridCuboid<PaletteVoxel, Shape>);

/// Create a default World with a grassy, diggable floor below level 0.
pub fn floor() -> World {
    let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, -8, 0].into());
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
    mut space: ResMut<World>,
    events: Res<Mutex<Receiver<Event>>>,
) {
    let events = events.try_lock();
    if let Ok(events) = events {
        for event in events.try_iter() {
            use Event::*;
            match event {
                LoadFile(path) => {
                    // Synchronous load.               
                    // This is an easy trick to avoid the user interacting with the world
                    // that is about to get replaced with a new one.
                    load(&mut space, path)
                        .unwrap_or_else(|e| eprintln!("Failed to load: {:?}", e));
                }
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

fn load(mut world: &mut World, path: PathBuf) -> Result<(), Box<dyn Error>>{
    let f = File::open(path)?;
    let mut f = BufReader::new(f);
    world.0 = bincode::deserialize_from(&mut f)?;
    Ok(())
}

fn save(world: World, path: PathBuf) -> Result<(), Box<dyn Error>>{
    let f = File::create(path)?;
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
    let space = FlatPaddedGridCuboid::<_, Shape>::new_from_space(&space, space.get_offset());
    
    let quads = generate_greedy_buffer_fast(&space);
    let material_lookup = |quad: &UnorientedQuad| {
        let i = space.get(space.get_offset() + VoxelUnits(to_i32_arr(quad.minimum))).0;
        let mut material = [0; 4];
        let i = i - 1; // 0 is empty
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

/// This needs to stay here because each BlockmeshShape is specific to each renderer.
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
                SystemSet::on_enter(EditorState::Loading)
                    .with_system(initialize_camera.system().after("load_chunks")),
            )
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


fn initialize_camera(mut commands: Commands, config: Res<Config>) {
    let eye = Vec3::new(40.0, 20.0, 40.0);
    let target = Vec3::new(20.0, 0.0, 20.0);
    camera::spawn(&mut commands, &(*config).camera, eye, target);
}

#[cfg(test)]
mod test {
//    use super::*;

}
