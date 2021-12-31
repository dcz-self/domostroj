/*! Stuff related to the world generator and its UI.
 *
 Based on bevy example source */
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

use baustein::prefab::World;
use baustein::world::Cow;

/// This creates a second window with a different camera
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_state(AppState::CreateWindow)
            .add_system_set(
                SystemSet::on_update(AppState::CreateWindow).with_system(setup_window.system()),
            )
            .add_system_set(SystemSet::on_update(AppState::Setup).with_system(setup_pipeline.system()));
    }
}

// NOTE: this "state based" approach to multiple windows is a short term workaround.
// Future Bevy releases shouldn't require such a strict order of operations.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    CreateWindow,
    Setup,
    Done,
}

fn setup_window(
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

    app_state.set(AppState::Setup).unwrap();
}

fn setup_pipeline(
    mut commands: Commands,
    windows: Res<Windows>,
    mut active_cameras: ResMut<ActiveCameras>,
    mut render_graph: ResMut<RenderGraph>,
    asset_server: Res<AssetServer>,
    msaa: Res<Msaa>,
    mut app_state: ResMut<State<AppState>>,
) {
    // get the non-default window id
    let window_id = windows
        .iter()
        .find(|w| w.id() != WindowId::default())
        .map(|w| w.id());

    let window_id = match window_id {
        Some(window_id) => window_id,
        None => return,
    };

    // here we setup our render graph to draw our second camera to the new window's swap chain

    // add a swapchain node for our new window
    render_graph.add_node(
        "second_window_swap_chain",
        WindowSwapChainNode::new(window_id),
    );

    // add a new depth texture node for our new window
    render_graph.add_node(
        "second_window_depth_texture",
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
    render_graph.add_system_node("secondary_camera", CameraNode::new("Generator"));

    // add a new render pass for our new window / camera
    let mut second_window_pass = PassNode::<&MainPass>::new(PassDescriptor {
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

    second_window_pass.add_camera("Generator");
    active_cameras.add("Generator");

    render_graph.add_node("second_window_pass", second_window_pass);

    render_graph
        .add_slot_edge(
            "second_window_swap_chain",
            WindowSwapChainNode::OUT_TEXTURE,
            "second_window_pass",
            if msaa.samples > 1 {
                "color_resolve_target"
            } else {
                "color_attachment"
            },
        )
        .unwrap();

    render_graph
        .add_slot_edge(
            "second_window_depth_texture",
            WindowTextureNode::OUT_TEXTURE,
            "second_window_pass",
            "depth",
        )
        .unwrap();

    render_graph
        .add_node_edge("secondary_camera", "second_window_pass")
        .unwrap();

    if msaa.samples > 1 {
        render_graph.add_node(
            "second_multi_sampled_color_attachment",
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
                "second_multi_sampled_color_attachment",
                WindowSwapChainNode::OUT_TEXTURE,
                "second_window_pass",
                "color_attachment",
            )
            .unwrap();
    }

    // SETUP SCENE

    // add entities to the world
    // light is shared between layers, sadly
    // generator window camera
    let eye = Vec3::new(40.0, 20.0, 40.0);
    let target = Vec3::new(20.0, 0.0, 20.0);
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            camera: Camera {
                name: Some("Generator".to_string()),
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

pub fn test_world() -> World {
    let world = World::default();
    let mut cow = Cow::new(&world);
    for x in -2..5 {
        for y in -2..5 {
            for z in -2..5 {
                cow.set([x, y, z].into(), 1)
            }
        }
    }
    let changes = cow.into_changes();
    let mut world = world;
    changes.apply(&mut world);
    world
}
