/*! Stuff related to the world generator and its UI.
 *
 Based on bevy example source. */
mod collapse;
pub mod render;
mod scene;
mod ui;

use crate::edit;
use crate::time;

use baustein;
use baustein::prefab::{ PaletteIdChunk, PaletteVoxel };
use baustein::re::ConstPow2Shape;
use baustein::world::{Cow, FlatPaddedGridCuboid};
use bevy::{
    prelude::*,
    render::{
        camera::{ActiveCameras, Camera, RenderLayers},
        pass::*,
        render_graph::{
            base::MainPass, CameraNode, PassNode, RenderGraph, WindowSwapChainNode,
            WindowTextureNode,
        },
        texture::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage},
    },
    window::{CreateWindow, WindowDescriptor, WindowId},
};

use bevy_egui;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use wfc_3d as wfc;
use wfc::wave;

// used traits
use baustein::traits::{ MutChunk, Space };


/// This creates a second window with a different camera
/// Requires: EguiPlugin
pub struct CameraPlugin;

impl bevy::app::Plugin for CameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_state(AppState::CreateWindow)
            .add_system_set(
                SystemSet::on_update(AppState::CreateWindow).with_system(setup_window.system()),
            )
            .add_system_set(SystemSet::on_update(AppState::Setup).with_system(setup_pipeline.system()));
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut AppBuilder) {
        let (ui_sender, ui_receiver) = mpsc::channel::<Event>();
        app
            .add_plugin(CameraPlugin)
            .insert_resource(scene::seed())
            .insert_resource(Mutex::new(ui_sender))
            .insert_resource(Mutex::new(ui_receiver))
            .insert_resource(StampsSource::None)
            .add_system_set(
                SystemSet::on_update(AppState::Done)
                    .with_system(ui::process.system())
                    .with_system(handle_events.system())
            )
            ;
    }
}

pub enum StampsSource {
    None,
    Present3x3x3(collapse::Stamps),
}

pub enum Event {
    StepOne,
    LoadStamps,
}

pub fn handle_events(
    source: Res<edit::World>,
    mut stamps: ResMut<StampsSource>,
    mut world: ResMut<scene::World>,
    events: Res<Mutex<Receiver<Event>>>,
) {
    let events = events.try_lock();
    if let Ok(events) = events {
        for event in events.try_iter() {
            use Event::*;
            match event {
                StepOne => match &*stamps {
                    StampsSource::None => {},
                    StampsSource::Present3x3x3(stamps) => {
                        let mut wave = &mut world.0;
                        // This should probably be relegated to another thread,
                        // but the other thread still needs mutable access to the same world
                        // that is being rendered and interacted with.
                        // That means a copy-on-write world should be used,
                        // because it lets the "base" reference read-only,
                        // but until that happens, the only alternative is to copy the whole world.
                        collapse::Stamps::rent(
                            stamps,
                            |stamps| {
                                // This is kind of expensive,
                                // but if we don't make sure all new collapses are resolved,
                                // then the entropy finder is going to ignore them and cause nonsense results.
                                if wave.collapse(&wave.get_extent(), stamps) == true {
                                    return;
                                }
                                let candidate = wfc::find_lowest_pseudo_entropy(
                                    wave.get_world(),
                                    stamps.get_distribution(),
                                    stamps.get_total_occurrences(),
                                );
                                println!("Stamp index: {:?}", candidate);
                                if let Some(index) = candidate {
                                    let stamp = wfc::find_preferred_stamp(
                                        wfc::stamp::ViewStamp::new(&wave.get_world(), index),
                                        &stamps,
                                    );
                                    println!("Stamp content: {:?}", stamp);
                                    // Trigger collapse
                                    wave.limit_stamp(index, &stamp, &stamps).unwrap();
                                }
                            },
                        );
                    },
                },
                LoadStamps => {
                    let converted_source
                        = source.0
                        .map(|v| v.0 as wfc::VoxelId)
                        .into();
                    *stamps = StampsSource::Present3x3x3(time!(
                        collapse::Stamps::from_source(converted_source)
                    ));
                }
            }
        }
    }
}
                    
pub struct Window(WindowId);

// NOTE: this "state based" approach to multiple windows is a short term workaround.
// Future Bevy releases shouldn't require such a strict order of operations.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    CreateWindow,
    Setup,
    Done,
}

fn setup_window(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut create_window_events: EventWriter<CreateWindow>,
) {
    let window_id = WindowId::new();

    // sends out a "CreateWindow" event, which will be received by the windowing backend
    create_window_events.send(CreateWindow {
        id: window_id,
        descriptor: WindowDescriptor {
            width: 800.,
            height: 600.,
            vsync: false,
            title: "Generator".to_string(),
            ..Default::default()
        },
    });
    commands.insert_resource(Window(window_id));

    app_state.set(AppState::Setup).unwrap();
}

mod window {
    pub const SWAP_CHAIN: &str = "generator_swap_chain";
    pub const DEPTH_TEXTURE: &str = "generator_depth_texture";
    pub const CAMERA_NODE: &str = "generator_camera";
    pub const CAMERA_NAME: &str = "Generator";
    pub const SAMPLED_COLOR_ATTACHMENT: &str = "generator_multi_sampled_color_attachment";
    pub const PASS: &str = "generator_window_pass";
}

fn setup_pipeline(
    mut commands: Commands,
    windows: Res<Windows>,
    window: Res<Window>,
    mut active_cameras: ResMut<ActiveCameras>,
    mut render_graph: ResMut<RenderGraph>,
    asset_server: Res<AssetServer>,
    msaa: Res<Msaa>,
    mut app_state: ResMut<State<AppState>>,
) {
    // For some reason the first call doesn't run with a registered window,
    // and crashes weirdly.
    if let None
        = windows.iter()
            .find(|w| w.id() == window.0)
    {
        return;
    }

    let window_id = window.0;

    // here we setup our render graph to draw our second camera to the new window's swap chain

    // add a swapchain node for our new window
    render_graph.add_node(
        window::SWAP_CHAIN,
        WindowSwapChainNode::new(window_id),
    );

    // add a new depth texture node for our new window
    render_graph.add_node(
        window::DEPTH_TEXTURE,
        WindowTextureNode::new(
            window_id,
            TextureDescriptor {
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::OUTPUT_ATTACHMENT,
                sample_count: msaa.samples,
                ..Default::default()
            },
        ),
    );

    // add a new camera node for our new window
    render_graph.add_system_node(
        window::CAMERA_NODE,
        CameraNode::new(window::CAMERA_NAME),
    );

    // add a new render pass for our new window / camera
    let mut pass = PassNode::<&MainPass>::new(PassDescriptor {
        color_attachments: vec![msaa.color_attachment_descriptor(
            TextureAttachment::Input("color_attachment".to_string()),
            TextureAttachment::Input("color_resolve_target".to_string()),
            Operations {
                load: LoadOp::Clear(Color::rgb(0.5, 0.5, 0.8)),
                store: true,
            },
        )],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
            attachment: TextureAttachment::Input("depth".to_string()),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
        sample_count: msaa.samples,
    });

    pass.add_camera(window::CAMERA_NAME);
    active_cameras.add(window::CAMERA_NAME);

    render_graph.add_node(window::PASS, pass);

    render_graph
        .add_slot_edge(
            window::SWAP_CHAIN,
            WindowSwapChainNode::OUT_TEXTURE,
            window::PASS,
            if msaa.samples > 1 {
                "color_resolve_target"
            } else {
                "color_attachment"
            },
        )
        .unwrap();

    render_graph
        .add_slot_edge(
            window::DEPTH_TEXTURE,
            WindowTextureNode::OUT_TEXTURE,
            window::PASS,
            "depth",
        )
        .unwrap();

    render_graph
        .add_node_edge(window::CAMERA_NODE, window::PASS)
        .unwrap();

    if msaa.samples > 1 {
        render_graph.add_node(
            window::SAMPLED_COLOR_ATTACHMENT,
            WindowTextureNode::new(
                window_id,
                TextureDescriptor {
                    size: Extent3d {
                        depth: 1,
                        width: 1,
                        height: 1,
                    },
                    mip_level_count: 1,
                    sample_count: msaa.samples,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::default(),
                    usage: TextureUsage::OUTPUT_ATTACHMENT,
                },
            ),
        );

        render_graph
            .add_slot_edge(
                window::SAMPLED_COLOR_ATTACHMENT,
                WindowSwapChainNode::OUT_TEXTURE,
                window::PASS,
                "color_attachment",
            )
            .unwrap();
    }

    bevy_egui::setup_pipeline(
        &mut render_graph,
        &msaa,
        bevy_egui::RenderGraphConfig {
            window_id,
            egui_pass: "egui_generator_pass",
            main_pass: window::PASS,
            swap_chain_node: window::SWAP_CHAIN,
            depth_texture: window::DEPTH_TEXTURE,
            sampled_color_attachment: window::SAMPLED_COLOR_ATTACHMENT,
            transform_node: "egui_generator_transform",
        },
    );

    // SETUP SCENE

    // add entities to the world
    // light is shared between layers, sadly
    // generator window camera
    let eye = Vec3::new(40.0, 20.0, 40.0);
    let target = Vec3::new(20.0, 0.0, 20.0);
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            camera: Camera {
                name: Some(window::CAMERA_NAME.to_string()),
                window: window_id,
                ..Default::default()
            },
            transform: eye_look_at_target_transform(eye, target),
            ..Default::default()
        })
        .insert(RenderLayers::layer(1));

    app_state.set(AppState::Done).unwrap();
}

fn eye_look_at_target_transform(eye: Vec3, target: Vec3) -> Transform {
    // If eye and target are very close, we avoid imprecision issues by keeping the look vector a unit vector.
    let look_vector = (target - eye).normalize();
    let look_at = eye + look_vector;

    Transform::from_translation(eye).looking_at(look_at, Vec3::Y)
}

pub fn test_world() -> baustein::prefab::World {
    let world = baustein::prefab::World::default();
    let mut cow = Cow::new(&world);
    for x in -2..5 {
        for y in -2..5 {
            for z in -2..5 {
                cow.set([x, y, z].into(), PaletteVoxel(1));
            }
        }
    }
    let changes = cow.into_changes();
    let mut world = world;
    changes.apply(&mut world);
    world
}

fn test_spinner() -> PaletteIdChunk {
    let mut chunk: PaletteIdChunk = Default::default();
    for x in 0..5 {
        for y in 0..2 {
            for z in 0..3 {
                chunk.set([x + 9, y + 9, z + 9].into(), PaletteVoxel(1));
            }
        }
    }
    chunk
}

pub fn create_test_spinner(
    mut commands: Commands,
) {
    commands.spawn()
        .insert(test_spinner())
        .insert(Transform::default());
}


pub fn spin_spinners(
    mut ts_spaces: Query<&mut Transform, With<PaletteIdChunk>>,
) {
    let rot_step = Transform::from_rotation(
        Quat::from_axis_angle([0.0, 1.0, 0.0].into(), 0.1)
    );
    for mut transform in ts_spaces.iter_mut() {
        *transform = transform.mul_transform(rot_step);
    }
}


/// A wrapper over a mundane chunk, for the purpose of becoming the Bevy resource.
#[derive(Clone)]
pub struct World(FlatPaddedGridCuboid<PaletteVoxel, ConstPow2Shape<5, 5, 5>>);

/// Create a default World with a grassy, diggable floor below level 0.
pub fn floor() -> World {
    let extent = FlatPaddedGridCuboid::<(), ConstPow2Shape<5, 5, 5>>::new([0, -8, 0].into());
    let world = extent.map_index(|i, _| {
        if i.y() < 0 {
            PaletteVoxel(2) // hopefully ground
        } else {
            PaletteVoxel::EMPTY
        }
    });
    World(world.into())
}


pub mod stress {
    use super::*;

    use baustein::re::ConstPow2Shape;
    use baustein::world::FlatPaddedGridCuboid;

    use crate::stress::Stress;

    type Shape = ConstPow2Shape<4, 4, 4>;

    fn test_stress() -> FlatPaddedGridCuboid::<Stress, Shape> {
        let mut chunk = FlatPaddedGridCuboid::new([-9, -9, -9].into());
        for x in 0..5 {
            for y in 0..2 {
                for z in 0..3 {
                    chunk.set([x, y, z].into(), Stress((x * y * z) as f32)).unwrap();
                }
            }
        }
        chunk
    }

    pub fn create_test_stress(
        mut commands: Commands,
    ) {
        commands.spawn()
            .insert(test_spinner())
            .insert(Transform::default());
    }
}

