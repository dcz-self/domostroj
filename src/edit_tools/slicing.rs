/*! Slicing mode: insert voxels on the specified plane only, hide anything that's above. */

pub mod user;

use crate::{geometry::offset_transform, CursorRay};
use crate::edit_tools::{VOXEL_WIDTH, CurrentTool};
use crate::geometry::{ ray_plane_intersection, Plane, RayPlaneIntersection };

use bevy::{
    asset::prelude::*,
    ecs::prelude::*,
    pbr::prelude::*,
    render::{
        mesh,
        pipeline::PrimitiveTopology,
        prelude::*,
    },
};
use bevy::math::Vec3;
use bevy::transform::components::Transform;
use feldspar::{
    bb::core::prelude::*,
};
use feldspar::prelude::MeshCutoff;

/// How far the hint box should extend towards the bottom.
const HINT_ANCHOR: f32 = -256.0;

/// A level indicator.
/// Like a water level marker on a river.
/// Extends from the bottom, and ends on the specified plane.
/// Follows cursor around, to indicate the current editing mode.
/// When switching to the selected mode,
/// it is automatically set on the plane where cursor's selection was.
/// Scale it using Transform, vertically.
pub struct SlicingHint;

/// The vertical index of the voxel,
/// which should be replaced,
/// and above which voxels should be made invisible.
pub struct SliceHeight(pub i32);

pub fn set_render_slice(
    slice_height: Res<SliceHeight>,
    current_tool: Res<CurrentTool>,
    mut render_cutoff: ResMut<MeshCutoff>,
) {
    *render_cutoff = if let CurrentTool::Slice = *current_tool {
        MeshCutoff(slice_height.0)
    } else {
        MeshCutoff::nothing()
    }
}

/* from Cobble */
/// The SliceHeight specifies the bottom of the voxel layer.
/// This places the hint where the cursor lies on the bottom of the voxel layer.
/// This looks cool if the layer is unoccupied: it's as if the plane was the floor.
/// But if the layer of voxels is occupied, they obscure the "floor",
/// so the cursor points at something below/inside the cube.
/// This is rather uncomforatble to use.
///
/// Solution 1: Use actual ray tracing to hit the voxel on its top or side if occupied.
/// This solution is a bunch of work though, obscures voxels.
/// Solution 2: Use a hint that's always visible, even if inside the mesh.
/// How to modify Z-order though?
pub fn update_slicing_hint(
    cursor_ray: Res<CursorRay>,
    current_tool: Res<CurrentTool>,
    slice_height: Res<SliceHeight>,
    mut hint: Query<
        (&mut Visible, &mut Transform),
        With<SlicingHint>,
    >,
) {
    if let Ok((mut draw, mut transform)) = hint.single_mut() {
        if let CurrentTool::Slice = *current_tool {
        } else {
            draw.is_visible = false;
            return;
        }
        
        draw.is_visible = if let CursorRay(Some(ray)) = *cursor_ray {
            // Everything past this point happens in the voxel coord system.
            let plane = Plane {
                origin: Vec3::new(0.0, 1.0 * slice_height.0 as f32, 0.0),
                normal: Vec3::new(0.0, 1.0, 0.0)
            };

            let intersection = ray_plane_intersection(&ray, &plane);
            if let RayPlaneIntersection::SinglePoint(point) = intersection {
                let voxel = PointN([point.x as i32, point.y as i32, point.z as i32]);

                let selection_layer_top = (slice_height.0 + 1) as f32 * VOXEL_WIDTH;
                let hint_height = selection_layer_top - HINT_ANCHOR;
                let voxel_position = VOXEL_WIDTH * Point3f::from(voxel);
                let hint_offset = PointN([voxel_position.x(), HINT_ANCHOR, voxel_position.z()]);
                *transform = Transform {
                    scale: Vec3::new(1.0, hint_height, 1.0),
                    ..offset_transform(hint_offset)
                };
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

pub fn setup_slicing_hint(
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
        .insert(SlicingHint);
}
