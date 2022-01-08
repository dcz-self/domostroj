/*! Rendering.
 * Depends on parts of feldspar's rendering pipeline.
 */
 
use baustein::indices::{to_i32_arr, ChunkIndex};
use baustein::prefab::{ PaletteIdChunk, PaletteVoxel, World };
use baustein::re;
use baustein::traits::{IterableSpace, Space};
use baustein::world::{ FlatPaddedGridCuboid, View };

use bevy::app;
use bevy::prelude::*;
use block_mesh;
use block_mesh::{visible_block_faces, UnitQuadBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG, UnorientedUnitQuad};
use feldspar::bb::mesh::PosNormMesh;
use feldspar::prelude::{
    spawn_array_material, ArrayMaterial, VoxelRenderAssets, VoxelType, VoxelTypeInfo, VoxelMaterial,
};
use feldspar::renderer::create_voxel_mesh_bundle;
use float_ord::FloatOrd;
use std::cmp;

use crate::stress::StressVoxel;

// Older version needed for block_mesh
type BlockMeshShape = block_mesh::ndshape::ConstShape3u32::<18, 18, 18>;


/// Requires: `LoadingTexture` resource.
pub struct Plugin;

impl app::Plugin for Plugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_state(TextureState::Loading)
            .add_system_set(
                SystemSet::on_enter(TextureState::Loading)
                    .with_system(start_loading_render_assets.system()),
            )
            .add_system_set(
                SystemSet::on_update(TextureState::Loading)
                    .with_system(wait_for_assets_loaded.system()),
            )
            .add_system_set(
                SystemSet::on_update(TextureState::Ready)
                    .with_system(update_meshes.system()),
            )
            ;
    }
}


/// Stores the material to be used for rendering.
/// Get one from `spawn_array_material`.
#[derive(Default)]
pub struct MeshMaterial(pub Handle<ArrayMaterial>);

impl From<Handle<ArrayMaterial>> for MeshMaterial {
    fn from(v: Handle<ArrayMaterial>) -> Self {
        Self(v)
    }
}

/// The value of stress.
#[derive(Clone, Copy)]
pub enum Voxel {
    Empty,
    Stressed(f32),
}

impl Default for Voxel {
    fn default() -> Self {
        Voxel::Empty
    }
}

impl block_mesh::Voxel for Voxel {
    fn is_empty(&self) -> bool {
        match self {
            Voxel::Empty => true,
            _ => false,
        }
    }
    fn is_opaque(&self) -> bool { !self.is_empty() }
}

pub type StressChunk = FlatPaddedGridCuboid<Voxel, re::ConstAnyShape<18, 18, 18>>;

/// Contains analyzed data to visualize.
pub struct Analyzed(pub StressChunk);

/// To track which parts should be despawned and when
pub struct StressMesh;

pub fn update_meshes(
    mut commands: Commands,
    mesh_material: Res<MeshMaterial>,
    spaces: Query<(&Analyzed, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_meshes: Query<Entity, With<StressMesh>>,
) {
    // Get rid of all meshes
    for cm in chunk_meshes.iter() {
        commands.entity(cm).despawn()
    }
    // And create the occupied ones again.
    // Wasteful, I know. I'm testing!
    for (space, transform) in spaces.iter() {
        let quads = generate_buffer_fast(&space.0);
        let material_lookup = |quad: &UnorientedUnitQuad| {
            let material = to_material(space.0.get(to_i32_arr(quad.minimum).into()));
            [material, material, material, material]
        };
        let mesh = mesh_from_quads(quads, &space.0, material_lookup);
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
                // This won't work within a hierarchy
                .insert(transform.clone())
                .insert(StressMesh)
                ;
        }
    }
}

/// Takes advantage of data structures
/// that already have samples in an array.
fn generate_buffer_fast<V, Shape>(
    view: &FlatPaddedGridCuboid<V, Shape>,
) -> UnitQuadBuffer
    where
    V: block_mesh::Voxel + Default + Copy,
    Shape: re::ConstShape,
{
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let samples = view.get_samples();
    let mut buffer = UnitQuadBuffer::new();

    visible_block_faces(
        samples,
        &BlockMeshShape {},
        [0, 0, 0],
        [
            <Shape as re::ConstShape>::ARRAY[0] as u32 - 1,
            <Shape as re::ConstShape>::ARRAY[1] as u32 - 1,
            <Shape as re::ConstShape>::ARRAY[2] as u32 - 1,
        ],
        &faces,
        &mut buffer,
    );
    buffer
}

pub fn mesh_from_quads<S, V, M, F>(
    buffer: UnitQuadBuffer,
    view: &S,
    mut vertex_map: F,
) -> Option<(PosNormMesh, Vec<M>)>
where
    M: Clone,
    S: Space<Voxel=V>,
    F: FnMut(&UnorientedUnitQuad) -> [M; 4],
{
    if buffer.num_quads() == 0 {
        None
    } else {
        // Build mesh from quads
        let num_indices = buffer.num_quads() * 6;
        let num_vertices = buffer.num_quads() * 4;
        let mut indices = Vec::with_capacity(num_indices);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut vertex_metadata = Vec::with_capacity(num_vertices);
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;
        for (group, face) in buffer.groups.iter().zip(faces.iter()) {
            for quad in group.into_iter() {
                indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                positions.extend_from_slice(&face.quad_mesh_positions(&((*quad).into()), 1.0));
                normals.extend_from_slice(&face.quad_mesh_normals());
                
                vertex_metadata.extend_from_slice(&vertex_map(&quad));
            }
        }

        Some((
            PosNormMesh {
                positions,
                normals,
                indices,
            },
            vertex_metadata,
        ))
    }
}

fn to_material(v: Voxel) -> [u8; 4] {
    match v {
        Voxel::Empty => [0; 4],
        Voxel::Stressed(v) => {
            let v = FloatOrd(v);
            let max = 256.0;
            let stress = cmp::max(v, FloatOrd(0.0));
            let stress = cmp::min(v, FloatOrd(max)).0;
            [stress as u8, (max - stress) as u8, 0, 0]
        },
    }
}

// Texture loading

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum TextureState {
    Loading,
    Ready,
}

/// A unique type for a resource containing the texture handle for this module.
pub struct LoadingTexture(pub Handle<Texture>);

fn start_loading_render_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(LoadingTexture(
        asset_server.load("stress.png"),
    ));
}

// From feldspar
fn wait_for_assets_loaded(
    mut commands: Commands,
    loading_texture: Res<LoadingTexture>,
    mut textures: ResMut<Assets<Texture>>,
    mut array_materials: ResMut<Assets<ArrayMaterial>>,
    mut state: ResMut<State<TextureState>>,
) {
    if textures.get(&loading_texture.0).is_some() {
        let params = VoxelRenderAssets {
            mesh_base_color: loading_texture.0.clone(),
            image_count: 4,
        };
        spawn_array_material::<MeshMaterial>(&params, commands, array_materials, textures);
        state.set(TextureState::Ready).unwrap();
    }
}


fn test_chunk() -> Analyzed {
    let mut chunk = StressChunk::new([0, 0, 0].into());
    for x in 0..5 {
        for y in 0..2 {
            for z in 0..3 {
                chunk.set([x + 9, y + 9, z + 9].into(), Voxel::Stressed(x as f32 * 50.0)).unwrap();
            }
        }
    }
    Analyzed(chunk)
}


pub fn spawn_test_chunk(
    mut commands: Commands,
) {
    commands.spawn()
        .insert(test_chunk())
        .insert(Transform::default());
}

