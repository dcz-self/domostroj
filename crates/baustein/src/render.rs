/*! Rendering.
 * Depends on parts of feldspar's rendering pipeline. */

use building_blocks_mesh::PosNormMesh;
use block_mesh::ndshape::ConstShape3u32;
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, RIGHT_HANDED_Y_UP_CONFIG};
use feldspar::prelude::{SdfVoxelPalette, VoxelType};
use feldspar::renderer::create_voxel_mesh_bundle;
use feldspar_core::glam::IVec3;
use feldspar_map::palette::PaletteId8;
use feldspar_map::units::VoxelUnits;

use crate::indices::{ to_i32_arr, to_u32_arr };
use crate::traits::{ChunkIndex, Space};
use crate::world::{World, View};


/*
fn generate_mesh_for_each_chunk(
    voxel_map: &SdfVoxelMap,
    dirty_chunks: &DirtyChunks,
    cutoff_height: &MeshCutoff,
    local_mesh_buffers: &ThreadLocalMeshBuffers,
    pool: &ComputeTaskPool,
) -> Vec<(ChunkKey3, Option<(PosNormMesh, Vec<[u8; 4]>)>)> {
*/

#[derive(Clone, Copy)]
struct PaletteVoxel(PaletteId8);

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


fn generate_mesh_for_chunk(
    world: &World,
    palette: &SdfVoxelPalette,
    index: ChunkIndex,
) -> Option<(PosNormMesh, Vec<[u8; 4]>)> {
    let wrapped = world.map(|v| PaletteVoxel(v));
    
    let view_offset = index.get_world_offset() - VoxelUnits(IVec3::new(1, 1, 1));
    let view = View::<_, ndshape::ConstShape3u32<18, 18, 18>>::new(
        &wrapped,
        view_offset,
    );

    let last = view.opposite_corner();

    let samples = view.clone().into_vec();
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = GreedyQuadsBuffer::new(samples.len());

    greedy_quads(
        &samples,
        &ViewShape {},
        to_u32_arr(view_offset.into()),
        to_u32_arr(last.into()),
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
