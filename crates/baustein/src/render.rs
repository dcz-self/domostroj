/*! Rendering.
 * Depends on parts of feldspar's rendering pipeline.
 * Most of this should be only suggested use,
 * and an inspiration for your own library.
 * Mesh generation is probably reuseable.
 *
 * TODO: which parts should end up in prefab?
 *
 * `feldspar::render` accepts an`ArrayMaterial` resource via `feldspar::renderer::create_voxel_mesh_bundle`,
 * and creates a `SmoothVoxelPbrBundle`.
 * 
 * To create the `ArrayMaterial`, provide a `LoadingTexture` resource and add the plugin.
 * When `State` reaches `Ready`, `ArrayMaterial` will be available as a resource.
 *
 * */
use bevy::app;
use bevy::prelude::*;
// Older version needed for block_mesh
use block_mesh::ndshape::ConstShape3u32;
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG, UnorientedQuad};
use feldspar::bb::mesh::PosNormMesh;
use feldspar::prelude::{
    spawn_array_material, ArrayMaterial, SdfVoxelPalette, VoxelRenderAssets, VoxelType, VoxelTypeInfo, VoxelMaterial,
};
use feldspar::renderer::create_voxel_mesh_bundle;
use ndshape::ConstShape;

use crate::indices::{to_i32_arr, ChunkIndex};
use crate::prefab::{ PaletteIdChunk, PaletteVoxel, World };
use crate::traits::{IterableSpace, Space};
use crate::world::{ Cow, View };


type ViewShape = ConstShape3u32::<18, 18, 18>;

/// Requires: `LoadingTexture` resource.
pub struct Plugin;

impl app::Plugin for Plugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .insert_resource(SdfVoxelPalette::new(vec![
                VoxelTypeInfo {
                    is_empty: true,
                    material: VoxelMaterial::NULL,
                },
                VoxelTypeInfo {
                    is_empty: false,
                    material: VoxelMaterial(0),
                },
                VoxelTypeInfo {
                    is_empty: false,
                    material: VoxelMaterial(1),
                },
                VoxelTypeInfo {
                    is_empty: false,
                    material: VoxelMaterial(2),
                },
                VoxelTypeInfo {
                    is_empty: false,
                    material: VoxelMaterial(3),
                },
            ]))
            .add_state(TextureState::Loading)
            .add_system_set(
                SystemSet::on_update(TextureState::Loading)
                    .with_system(wait_for_assets_loaded.system()),
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


/// To track which parts should be despawned and when
pub struct ChunkMesh;

pub fn generate_meshes(
    mut commands: Commands,
    world: Res<World>,
    palette: Res<SdfVoxelPalette>,
    //cutoff_height: Res<MeshCutoff>,
    mesh_material: Res<MeshMaterial>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_meshes: Query<Entity, With<ChunkMesh>>,
) {
    // Get rid of all meshes
    for cm in chunk_meshes.iter() {
        commands.entity(cm).despawn()
    }
    // And create the occupied ones again.
    // Wasteful, I know. I'm testing!
    for index in world.iter_chunk_indices() {
        let mesh = generate_mesh_for_chunk(&world, &palette, index);
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
                .insert(Transform::from_translation(
                    index
                        .get_world_offset()
                        .into()
                ))
                .insert(ChunkMesh)
                ;
        }
    }
}

/// A space which is affected by the Transform component before meshing.
pub struct TransformMesh;

pub fn generate_transformeshes(
    mut commands: Commands,
    palette: Res<SdfVoxelPalette>,
    //cutoff_height: Res<MeshCutoff>,
    mesh_material: Res<MeshMaterial>,
    ts_spaces: Query<(&PaletteIdChunk, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    tf_meshes: Query<Entity, With<TransformMesh>>,
) {
    // Get rid of all meshes
    for cm in tf_meshes.iter() {
        commands.entity(cm).despawn()
    }

    for (space, transform) in ts_spaces.iter() {
        let mut world = World::default();
        let mut overlay = Cow::new(&world);
        let centered = View::<_, ndshape::ConstShape3u32<18, 18, 18>>::new(
            space,
            [9, 9, 9].into(),
        );
        // This is kind of lousy, may have holes inside.
        // It would have been better to perform the inverse transformation,
        // but the API would have to expose source's occupancy.
        centered.visit_indices(|index| {
            let voxel = centered.get(index);
            let tf_index = transform.mul_vec3(index.into());
            if voxel != PaletteVoxel::EMPTY {
                overlay.set(tf_index.into(), voxel);
            }
        });
        let changes = overlay.into_changes();
        changes.apply(&mut world);
        // Screw accuracy. Alien chunks can only be close to the middle.
        let view = View::<_, ndshape::ConstShape3u32<18, 18, 18>>::new(
            &world,
            [-9, -9, -9].into(),
        );
        let quads = generate_greedy_buffer(view.clone());

        let material_lookup = |quad: &UnorientedQuad| {
            let material = to_material(&palette, view.get(to_i32_arr(quad.minimum).into()));
            [material, material, material, material]
        };

        let mesh = mesh_from_quads(quads, &view, material_lookup);
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
                .insert(TransformMesh)
                ;
        }
    }
}

fn generate_greedy_buffer<V, S, Shape>(
    view: View<S, Shape>,
) -> GreedyQuadsBuffer
    where
    V: MergeVoxel,
    S: Space<Voxel=V>,
    Shape: ConstShape<3, Coord=u32>
{
    let samples = view.into_vec();
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = GreedyQuadsBuffer::new(samples.len());

    greedy_quads(
        &samples,
        &ViewShape {},
        [0, 0, 0],
        [Shape::ARRAY[0] - 1, Shape::ARRAY[1] - 1, Shape::ARRAY[2] - 1],
        &faces,
        &mut buffer,
    );
    buffer
}

fn generate_mesh_for_chunk(
    world: &World,
    palette: &SdfVoxelPalette,
    index: ChunkIndex,
) -> Option<(PosNormMesh, Vec<[u8; 4]>)> {
    let view_offset = index.get_world_offset();
    let view = View::<_, ndshape::ConstShape3u32<18, 18, 18>>::new(
        &world,
        view_offset,
    );

    let samples = view.clone().into_vec();
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = GreedyQuadsBuffer::new(samples.len());

    greedy_quads(
        &samples,
        &ViewShape {},
        [0, 0, 0],
        [17, 17, 17],
        &faces,
        &mut buffer,
    );
    
    let material_lookup = |quad: &UnorientedQuad| {
        let material = to_material(&palette, view.get(to_i32_arr(quad.minimum).into()));
        [material, material, material, material]
    };
    
    mesh_from_quads(buffer, &view, material_lookup)
}

pub fn mesh_from_quads<S, V, M, F>(
    buffer: GreedyQuadsBuffer,
    view: &S,
    mut vertex_map: F,
) -> Option<(PosNormMesh, Vec<M>)>
where
    M: Clone,
    S: Space<Voxel=V>,
    F: FnMut(&UnorientedQuad) -> [M; 4],
{
    if buffer.quads.num_quads() == 0 {
        None
    } else {
        // Build mesh from quads
        let num_indices = buffer.quads.num_quads() * 6;
        let num_vertices = buffer.quads.num_quads() * 4;
        let mut indices = Vec::with_capacity(num_indices);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut vertex_metadata = Vec::with_capacity(num_vertices);
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;
        for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
            for quad in group.into_iter() {
                indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                positions.extend_from_slice(&face.quad_mesh_positions(&quad, 1.0));
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

fn to_material(palette: &SdfVoxelPalette, v: PaletteVoxel) -> [u8; 4] {
    let mut materials = [0; 4];
    let info = palette.get_voxel_type_info(VoxelType(v.0));
    materials[info.material.0 as usize] = 1;
    materials
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum TextureState {
    Loading,
    Ready,
}

/// A unique type for a resource containing the texture handle for this module.
pub struct LoadingTexture(pub Handle<Texture>);

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
