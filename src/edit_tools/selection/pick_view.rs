use super::SelectionState;

use crate::{geometry::offset_transform, ImmediateModeTag, VoxelCursorRayImpact};
use crate::edit_tools::VOXEL_WIDTH;
use crate::edit_tools::CurrentTool;

use bevy::{
    asset::prelude::*,
    ecs::prelude::*,
    pbr::prelude::*,
    render::{
        mesh,
        mesh::{VertexAttributeValues},
        pipeline::PrimitiveTopology,
        prelude::*,
    },
};
use bevy::transform::components::Transform;
use feldspar::{
    bb::core::prelude::*,
};
use feldspar::bb::mesh::{OrientedCubeFace, PosNormMesh, UnorientedQuad};


pub struct SelectionTag;

// TODO: those will never be larger than 1px, so merge into pick_hint
fn initialize_pick_view(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut color = Color::YELLOW;
    color.set_a(0.5);
    let material = SelectionCursorMaterial(materials.add(StandardMaterial::from(color)));
    commands.insert_resource(material);
}

fn pick_view_system(
    selection_state: Res<SelectionState>,
    cursor_voxel: Res<VoxelCursorRayImpact>,
    material: Res<SelectionCursorMaterial>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if let Some(voxel_face) = cursor_voxel.get_voxel_face() {
        let quad = UnorientedQuad::from_voxel(voxel_face.point);
        let face = OrientedCubeFace::canonical(voxel_face.normal);

        create_quad_pick_hint_entity(
            &quad,
            &face,
            material.0.clone(),
            &mut commands,
            &mut *meshes,
        );
    }
}

pub struct SelectionCursorMaterial(pub Handle<StandardMaterial>);

fn create_quad_pick_hint_entity(
    quad: &UnorientedQuad,
    face: &OrientedCubeFace,
    material: Handle<StandardMaterial>,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
) -> Entity {
    commands
        .spawn_bundle(create_single_quad_mesh_bundle(
            &face, material, meshes,
        ))
        .insert(offset_transform(1.0 * Point3f::from(quad.minimum)))
        .insert(ImmediateModeTag)
        .id()
}

fn create_single_quad_mesh_bundle(
    face: &OrientedCubeFace,
    material: Handle<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> PbrBundle {
    let mut mesh = PosNormMesh::default();
    face.add_quad_to_pos_norm_mesh(&UnorientedQuad::from_voxel(PointN([0, 0, 0])), 1.0, &mut mesh);

    let num_vertices = mesh.positions.len();

    let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    render_mesh.set_attribute(
        "Vertex_Position",
        VertexAttributeValues::Float3(mesh.positions),
    );
    render_mesh.set_attribute("Vertex_Normal", VertexAttributeValues::Float3(mesh.normals));
    // HACK: We have to provide UVs, even though we don't use them.
    render_mesh.set_attribute(
        "Vertex_Uv",
        VertexAttributeValues::Float2(vec![[0.0; 2]; num_vertices]),
    );
    render_mesh.set_indices(Some(mesh::Indices::U32(mesh.indices)));

    PbrBundle {
        mesh: meshes.add(render_mesh),
        material,
        ..Default::default()
    }
}

/* from Cobble */
pub fn update_pick_hint(
    cursor_voxel: Res<VoxelCursorRayImpact>,
    current_tool: Res<CurrentTool>,
    mut query_selection: Query<
        (&mut Visible, &mut Transform),
        With<SelectionTag>,
    >,
) {
    if let Ok((mut draw, mut transform)) = query_selection.single_mut() {
        let cond = (cursor_voxel.get_neighoring_voxel(), &*current_tool);
        if let (Some(voxel), &CurrentTool::Terraform) = cond {
            *transform = offset_transform(VOXEL_WIDTH * Point3f::from(voxel));
            draw.is_visible = true;
        } else {
            draw.is_visible = false;
        }
    }
}

pub fn setup_pick_hint(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const VERTICES: [([f32; 3], [f32; 3], [f32; 2]); 8] = [
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.0, 1.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 1.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0]),
    ];
    let indices = mesh::Indices::U32(vec![
        0, 1, 0, 2, 0, 3, 7, 4, 7, 5, 7, 6, 1, 5, 1, 4, 2, 4, 2, 6, 3, 5, 3, 6,
    ]);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (position, normal, uv) in VERTICES.iter() {
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::LineList);
    mesh.set_indices(Some(indices));
    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            mesh: meshes.add(mesh.clone()),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb_linear(1.0, 1.0, 1.0),
                double_sided: false,
                unlit: true,
                ..Default::default()
            }),
            ..Default::default()
        })
        .insert(SelectionTag);
}
