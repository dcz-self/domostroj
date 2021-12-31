/*! Rendering.
 * Depends on parts of feldspar's rendering pipeline. */
use bevy::app;
use bevy::prelude::*;
use block_mesh::ndshape::ConstShape3u32;
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, RIGHT_HANDED_Y_UP_CONFIG};
use feldspar::bb::mesh::PosNormMesh;
use feldspar::prelude::{ChunkMeshes, MeshMaterial, SdfVoxelPalette, VoxelType, VoxelTypeInfo, VoxelMaterial};
use feldspar::renderer::create_voxel_mesh_bundle;
use feldspar_core::glam::IVec3;
use feldspar_map::palette::PaletteId8;
use feldspar_map::units::VoxelUnits;
use ndshape::ConstShape;

use crate::indices::to_i32_arr;
use crate::prefab::{ PaletteIdChunk, PaletteVoxel, World };
use crate::traits::{ChunkIndex, IterableSpace, Space};
use crate::world::{ Cow, View };


type ViewShape = ConstShape3u32::<18, 18, 18>;


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
            ]));
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
    mut chunk_meshes: Query<Entity, With<ChunkMesh>>,
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
    mut tf_meshes: Query<Entity, With<TransformMesh>>,
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
                //dbg!(index);
                overlay.set(tf_index.into(), voxel);
            }
        });
        let changes = overlay.into_changes();
        changes.apply(&mut world);
        //dbg!(world.get([13, 10, 11].into()));
        // Screw accuracy. Alien chunks can only be close to the middle.
        let view = View::<_, ndshape::ConstShape3u32<18, 18, 18>>::new(
            &world,
            [-9, -9, -9].into(),
        );
        let quads = generate_greedy_buffer(view.clone());

        let mesh = mesh_from_quads(quads, &palette, view);
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
    //panic!()
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
    mesh_from_quads(buffer, palette, view)
}

fn mesh_from_quads<S: Space<Voxel=PaletteVoxel>>(
    buffer: GreedyQuadsBuffer,
    palette: &SdfVoxelPalette,
    view: S,
) -> Option<(PosNormMesh, Vec<[u8; 4]>)> {
    if buffer.quads.num_quads() == 0 {
        None
    } else {
        // Build mesh from quads
        let num_indices = buffer.quads.num_quads() * 6;
        let num_vertices = buffer.quads.num_quads() * 4;
        let mut indices = Vec::with_capacity(num_indices);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut materials = Vec::with_capacity(num_vertices);
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;
        for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
            for quad in group.into_iter() {
                indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                positions.extend_from_slice(&face.quad_mesh_positions(&quad, 1.0));
                normals.extend_from_slice(&face.quad_mesh_normals());
                let material = to_material(&palette, view.get(to_i32_arr(quad.minimum).into()));
                materials.extend_from_slice(&[material, material, material, material]);
            }
        }

        Some((
            PosNormMesh {
                positions,
                normals,
                indices,
            },
            materials,
        ))
    }
}


fn to_material(palette: &SdfVoxelPalette, v: PaletteVoxel) -> [u8; 4] {
    let mut materials = [0; 4];
    let info = palette.get_voxel_type_info(VoxelType(v.0));
    materials[info.material.0 as usize] = 1;
    materials
}
