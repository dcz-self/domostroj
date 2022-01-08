/*! Edit mode:
 * world, rendering, UI. */
use baustein::indices::to_i32_arr;
use baustein::prefab::PaletteVoxel;
use baustein::re::{ ConstPow2Shape, ConstShape };
use baustein::render::{ mesh_from_quads, MeshMaterial };
use baustein::traits::Space;
use baustein::world::FlatPaddedGridCuboid;
use bevy::asset::Assets;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::system::{ Commands, Query, Res, ResMut };
use bevy::render::mesh::Mesh;
use bevy::transform::components::Transform;
use block_mesh;
use block_mesh::{ greedy_quads, GreedyQuadsBuffer, MergeVoxel, UnorientedQuad, RIGHT_HANDED_Y_UP_CONFIG };
use feldspar::prelude::create_voxel_mesh_bundle;


use baustein::traits::Extent;


/// A wrapper over a mundane chunk, for the purpose of becoming the Bevy resource.
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

// Older version needed for block_mesh
type BlockMeshShape = block_mesh::ndshape::ConstPow2Shape3u32::<5, 5, 5>;

/// Marks which meshes should despawn
pub struct EditMesh;

pub fn update_meshes(
    mut commands: Commands,
    mesh_material: Res<MeshMaterial>,
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
    
    let quads = generate_greedy_buffer_fast(&space.0);
    let material_lookup = |quad: &UnorientedQuad| {
        let i = space.0.get(to_i32_arr(quad.minimum).into()).0;
        let mut material = [0; 4];
        material[i as usize] = 1;
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
            .insert(Transform::from_translation(space.0.get_offset().into()))
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
