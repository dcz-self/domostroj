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
use std::fmt;

use crate::indices::to_i32_arr;
use crate::traits::{ChunkIndex, Space};
use crate::world::{World, View};


#[derive(Clone, Copy, PartialEq)]
struct PaletteVoxel(PaletteId8);

impl fmt::Debug for PaletteVoxel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl Voxel for PaletteVoxel {
    fn is_empty(&self) -> bool {
        self.0 == 0
    }
    fn is_opaque(&self) -> bool {
        self.0 != 0
    }
}

impl MergeVoxel for PaletteVoxel {
    type MergeValue = u8;
    fn merge_value(&self) -> Self::MergeValue {
        self.0
    }
}

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


fn generate_mesh_for_chunk(
    world: &World,
    palette: &SdfVoxelPalette,
    index: ChunkIndex,
) -> Option<(PosNormMesh, Vec<[u8; 4]>)> {
    let wrapped = world.map(|v| PaletteVoxel(v));
    
    let view_offset = index.get_world_offset();
    let view = View::<_, ndshape::ConstShape3u32<18, 18, 18>>::new(
        &wrapped,
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
