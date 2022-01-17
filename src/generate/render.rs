/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */
use baustein::indices::{to_i32_arr, VoxelUnits};
use baustein::prefab::PaletteVoxel;
use baustein::re::{ ConstPow2Shape, ConstShape };
use baustein::render::{ generate_greedy_buffer_fast, mesh_from_quads, MeshMaterial };
use baustein::traits::Space;
use baustein::world::FlatPaddedGridCuboid;
use bevy::app;
use bevy::app::AppBuilder;
use bevy::asset::{ Assets, AssetServer, Handle };
use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::schedule::{ State, SystemSet };
use bevy::ecs::system::{ Commands, Query, Res, ResMut };
use bevy::math::Vec3;
use bevy::render::camera::RenderLayers;
use bevy::render::mesh::Mesh;
use bevy::render::texture::Texture;
use bevy::transform::components::Transform;
use block_mesh;
use block_mesh::{ greedy_quads, GreedyQuadsBuffer, MergeVoxel, UnorientedQuad, RIGHT_HANDED_Y_UP_CONFIG };
use feldspar::prelude::{ create_voxel_mesh_bundle, spawn_array_material, ArrayMaterial, VoxelRenderAssets};

use crate::generate::scene;
use crate::generate::scene::{SceneShape, World};

// Used traits
use baustein::traits::Cuboid as Extent;
use bevy::prelude::IntoSystem;


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

// Older version needed for block_mesh
type BlockMeshShape = SceneShape;

/// Marks which meshes should despawn
pub struct MeshTag;

#[derive(Eq, Clone, Copy, Default, Debug)]
struct Voxel(scene::Superposition);

impl PartialEq for Voxel {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl MergeVoxel for Voxel {
    type MergeValue = Self;
    fn merge_value(&self) -> Self {
        *self
    }
}

impl block_mesh::Voxel for Voxel {
    fn is_empty(&self) -> bool {
        self.0.allows(scene::Voxel::Empty)
    }
    fn is_opaque(&self) -> bool {
        !self.0.allows(scene::Voxel::Empty)
    }
}

/// Shows only voxels that collapsed into solid.
fn to_material_empty(v: Voxel) -> [u8; 4] {
    use scene::Voxel::*;
    if v.0.allows(Empty) {
        [0; 4]
    } else {
        let f = |t| {
            if v.0.allows(t) { 1 }
            else { 0 }
        };
        [f(Grass), f(Concrete), f(Wood), f(Glass)]
    }
}

pub fn update_meshes(
    mut commands: Commands,
    mesh_material: Res<MeshMaterial>,
    space: Res<World>,
    mut meshes: ResMut<Assets<Mesh>>,
    edit_meshes: Query<Entity, With<MeshTag>>,
) {
    // Get rid of all meshes
    for cm in edit_meshes.iter() {
        commands.entity(cm).despawn()
    }
    // And create the occupied ones again.
    // Wasteful, I know. I'm testing!
    let space = space.wave.get_world().map(|v| Voxel(v.into()));
    let space = FlatPaddedGridCuboid::<Voxel, SceneShape>::new_from_space(&space, space.get_offset());
    let quads = generate_greedy_buffer_fast(&space);
    let material_lookup = |quad: &UnorientedQuad| {
        let v = space.get(space.get_offset() + VoxelUnits(to_i32_arr(quad.minimum)));
        let material = to_material_empty(v);
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
            .insert(MeshTag)
            .insert(RenderLayers::layer(1))
            ;
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum TextureState {
    Loading,
    Ready,
}

/// A unique type for a resource containing the texture handle for this module.
pub struct LoadingTexture(pub Handle<Texture>);

fn start_loading_render_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(LoadingTexture(
        asset_server.load("grass_rock_snow_dirt/base_color.png"),
    ));
}

// From feldspar
fn wait_for_assets_loaded(
    commands: Commands,
    loading_texture: Res<LoadingTexture>,
    textures: ResMut<Assets<Texture>>,
    array_materials: ResMut<Assets<ArrayMaterial>>,
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
